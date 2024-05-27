use crate::db_notification::{self, Notifier};
use crate::nino_constants::{info, USE_RESPONSE_CACHE};
use crate::nino_functions;
use crate::nino_structures::ServletTask;
use crate::web_requests::RequestInfo;
use crate::{
    db::DBManager,
    nino_constants,
    nino_structures::{self, JSTask},
};
use async_channel::{Receiver, Sender};
use async_std::net::TcpStream;
use deno_runtime::deno_core::anyhow::Error;
use http_types::{Mime, Request, Response, StatusCode};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, OnceLock, RwLock};

#[derive(Clone)]
pub struct ResponseManager {
    db: Arc<DBManager>,
    js_thread_count: usize,
    web_task_sx: Sender<nino_structures::JSTask>,
    web_task_rx: Receiver<nino_structures::JSTask>,
    notifier: Arc<Notifier>,
}

#[derive(Clone)]
pub struct ResponseInfo {
    pub mime: Mime,
    pub execute: bool,
    pub transpile: bool,
}

static RESPONSE_CACHE: OnceLock<RwLock<HashMap<String, ResponseInfo>>> = OnceLock::new();

impl ResponseManager {
    pub fn new(
        db: Arc<DBManager>,
        js_thread_count: usize,
        notifier: Arc<Notifier>,
        db_subscribe: tokio::sync::broadcast::Receiver<nino_structures::NotificationMessage>,
    ) -> ResponseManager {
        RESPONSE_CACHE.get_or_init(|| RwLock::new(HashMap::new()));
        // web_task channel is used to send tasks to the js threads
        let (web_task_sx, web_task_rx) = async_channel::unbounded::<nino_structures::JSTask>();
        let this = Self {
            db,
            js_thread_count,
            web_task_sx,
            web_task_rx,
            notifier,
        };
        let thizz = this.clone();
        tokio::spawn(async move {
            thizz.reload_responses().await;
            thizz.invalidator(db_subscribe).await;
        });
        this
    }

    pub fn get_notifier(&self) -> Arc<Notifier> {
        self.notifier.clone()
    }

    pub fn get_web_task_rx(&self) -> Receiver<JSTask> {
        self.web_task_rx.clone()
    }

    pub async fn invalidator(
        &self,
        mut db_subscribe: tokio::sync::broadcast::Receiver<nino_structures::NotificationMessage>,
    ) {
        loop {
            match db_subscribe.recv().await {
                Err(error) => {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                }
                Ok(message) => {
                    info!("MSG:responses: {}", message.text);

                    if message
                        .text
                        .starts_with(db_notification::NOTIFICATION_PREFIX_RESPONSE)
                    {
                        //reload the db aliases
                        self.reload_responses().await;
                    }

                    // send invalidation messages to the js threads
                    let web_task = nino_structures::JSTask::Message(message.text);
                    for _ in 0..self.js_thread_count {
                        if let Err(error) = self.web_task_sx.send(web_task.clone()).await {
                            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                        }
                    }
                }
            }
        }
    }

    async fn reload_responses(&self) {
        //reload the db aliases
        let query: String = format!(
            "SELECT response_name, response_mime_type, execute_flag, transpile_flag FROM {}",
            nino_constants::RESPONSE_TABLE
        );

        match self.db.query(&query, &[]).await {
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            }
            Ok(rows) => {
                let mut map = RESPONSE_CACHE.get().unwrap().write().unwrap();
                map.clear();
                for row in rows {
                    let name: String = row.get(0);
                    let mime_str: String = row.get(1);
                    let mime = Mime::from_str(&mime_str).unwrap();
                    let execute: bool = row.get(2);
                    let transpile: bool = row.get(3);
                    map.insert(
                        name,
                        ResponseInfo {
                            mime,
                            execute,
                            transpile,
                        },
                    );
                }
            }
        }
    }

    pub async fn get_response(&self, name: &str) -> Result<Option<ResponseInfo>, Error> {
        if USE_RESPONSE_CACHE {
            let map = RESPONSE_CACHE.get().unwrap().read().unwrap();
            Ok(map.get(name).cloned())
        } else {
            let query: String = format!(
                "SELECT response_mime_type, execute_flag, transpile_flag FROM {} WHERE response_name = $1",
                nino_constants::RESPONSE_TABLE
            );
            let result = self.db.query_opt(&query, &[&name]).await?;
            match result {
                None => Ok(None),
                Some(row) => {
                    let mime_str: String = row.get(0);
                    let mime = Mime::from_str(&mime_str).unwrap();
                    let execute: bool = row.get(1);
                    let transpile: bool = row.get(2);
                    Ok(Some(ResponseInfo {
                        mime,
                        execute,
                        transpile,
                    }))
                }
            }
        }
    }

    // returns the response
    pub async fn get_response_bytes(&self, name: &str) -> Result<Vec<u8>, Error> {
        let query: String = format!(
            "SELECT response_content FROM {} WHERE response_name = $1",
            nino_constants::RESPONSE_TABLE
        );
        let row = self.db.query_opt(&query, &[&name]).await?;
        match row {
            None => Err(Error::msg(format!(
                "response '{}' does not exist in database",
                name
            ))),
            Some(row) => {
                let js_bytes: Vec<u8> = row.get(0);
                Ok(js_bytes)
            }
        }
    }

    // returns the transpiled code of the response
    pub async fn get_response_javascript(&self, name: &str) -> Result<Vec<u8>, Error> {
        let query: String = format!(
            "SELECT (CASE WHEN transpile_flag THEN javascript ELSE response_content END) FROM {} WHERE response_name = $1",
            nino_constants::RESPONSE_TABLE
        );
        let row = self.db.query_opt(&query, &[&name]).await?;
        match row {
            None => Err(Error::msg(format!(
                "response '{}' does not exist in database",
                name
            ))),
            Some(row) => {
                let js_bytes: Vec<u8> = row.get(0);
                Ok(js_bytes)
            }
        }
    }

    pub async fn serve_dynamic(
        &self,
        method: String,
        request_path: String,
        request: Request,
        request_info: &RequestInfo,
        response_info: &ResponseInfo,
        stream: Box<TcpStream>,
        user: String,
        body: String,
    ) -> Result<(), Error> {
        // default response
        let mut response = Response::new(200);
        let js_module = request_info.name.clone();
        response.set_content_type(response_info.mime.clone());
        //send new task to the javascript threads
        let js_task_request = ServletTask {
            method,
            request_path,
            js_module,
            request,
            user,
            body,
            stream,
            response,
        };
        let web_task = nino_structures::JSTask::Servlet(js_task_request);
        self.web_task_sx.send(web_task).await?;
        Ok(())
    }

    pub async fn serve_static(
        &self,
        request_info: RequestInfo,
        response_info: ResponseInfo,
        stream: Box<TcpStream>,
    ) -> Result<(), Error> {
        // serve content
        let content = self.get_response_javascript(&request_info.name).await?;
        let mut response = Response::new(StatusCode::Ok);
        response.set_content_type(response_info.mime);
        response.set_body(http_types::Body::from(content));
        nino_functions::send_response_to_stream(stream, &mut response).await?;
        Ok(())
    }
}

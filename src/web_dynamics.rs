use crate::db_notification::{self, Notifier};
use crate::nino_constants::{info, USE_DYNAMIC_CACHE};
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
use deno_runtime::deno_core::anyhow::{self, Error};
use http_types::{Request, Response, StatusCode};
use std::collections::HashSet;
use std::sync::{Arc, OnceLock, RwLock};

#[derive(Clone)]
pub struct DynamicManager {
    db: Arc<DBManager>,
    js_thread_count: usize,
    web_task_sx: Sender<nino_structures::JSTask>,
    web_task_rx: Receiver<nino_structures::JSTask>,
    notifier: Arc<Notifier>,
}

static DYNAMIC_CACHE: OnceLock<RwLock<HashSet<String>>> = OnceLock::new();

impl DynamicManager {
    pub fn new(
        db: Arc<DBManager>,
        js_thread_count: usize,
        notifier: Arc<Notifier>,
        db_subscribe: tokio::sync::broadcast::Receiver<nino_structures::NotificationMessage>,
    ) -> DynamicManager {
        DYNAMIC_CACHE.get_or_init(|| RwLock::new(HashSet::new()));
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
            thizz.reload_dynamics().await;
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
                    info!("MSG:dymnamics: {}", message.text);

                    if message
                        .text
                        .starts_with(db_notification::NOTIFICATION_PREFIX_DYNAMICS)
                    {
                        //reload the db aliases
                        self.reload_dynamics().await;
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

    async fn reload_dynamics(&self) {
        //reload the db aliases
        let query: String = format!(
            "SELECT dynamic_name FROM {} ORDER BY dynamic_name",
            nino_constants::DYNAMICS_TABLE
        );
        match self.db.query(&query, &[]).await {
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            }
            Ok(rows) => {
                let mut map = DYNAMIC_CACHE.get().unwrap().write().unwrap();
                map.clear();
                for row in rows {
                    let dynamic_name: String = row.get(0);
                    map.insert(dynamic_name);
                }
            }
        }
    }

    async fn is_dynamic_path_exists(&self, path: &str) -> bool {
        if USE_DYNAMIC_CACHE {
            let map = DYNAMIC_CACHE.get().unwrap().read().unwrap();
            return map.contains(path);
        }
        let query: String = format!(
            "SELECT dynamic_name FROM {} WHERE dynamic_name = $1",
            nino_constants::DYNAMICS_TABLE
        );
        match self.db.query_opt(&query, &[&path]).await {
            Ok(row) => match row {
                None => false,
                Some(_) => true,
            },
            Err(_) => false,
        }
    }

    // returns the dynamic original code (before transpiling)
    pub async fn get_module_code(&self, name: &str) -> Result<String, Error> {
        let query: String = format!(
            "SELECT code FROM {} WHERE dynamic_name = $1",
            nino_constants::DYNAMICS_TABLE
        );
        let row = self.db.query_opt(&query, &[&name]).await?;
        match row {
            None => Err(Error::msg(format!(
                "dynamic '{}' does not exist in database",
                name
            ))),
            Some(row) => {
                let js_bytes: Vec<u8> = row.get(0);
                let js = String::from_utf8(js_bytes).unwrap();
                Ok(js)
            }
        }
    }

    // returns the dynamic transpiled code
    pub async fn get_module_javascript(&self, path: &str) -> Result<String, Error> {
        let query: String = format!(
            "SELECT javascript FROM {} WHERE dynamic_name = $1",
            nino_constants::DYNAMICS_TABLE
        );
        let row = self.db.query_opt(&query, &[&path]).await?;
        match row {
            None => Err(Error::msg(format!(
                "dynamic '{}' does not exist in database",
                path
            ))),
            Some(row) => {
                let js_bytes: Vec<u8> = row.get(0);
                let js = String::from_utf8(js_bytes)?;
                Ok(js)
            }
        }
    }

    pub async fn serve_dynamic(
        &self,
        request_info: RequestInfo,
        stream: Box<TcpStream>,
    ) -> Result<(), Error> {
        // look for matching path
        let js_module = self.get_module_javascript(&request_info.name).await?;
        let mut response = Response::new(StatusCode::Ok);
        response.set_content_type(request_info.mime);
        response.set_body(http_types::Body::from(js_module));
        nino_functions::send_response_to_stream(stream, &mut response).await
    }

    pub async fn execute_dynamic(
        &self,
        request_info: RequestInfo,
        request: Request,
        stream: Box<TcpStream>,
        user: String,
        body: String,
    ) -> Result<(), Error> {
        // look for matching path
        let exists = self.is_dynamic_path_exists(&request_info.name).await;
        if !exists {
            return Err(anyhow::anyhow!(
                "dynamic name does not exist: {}",
                request_info.name
            ));
        }
        // default response
        let mut response = Response::new(200);
        let js_module = request_info.name.clone();
        response.set_content_type(request_info.mime);
        //send new task to the javascript threads
        let js_task_request = ServletTask {
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
}

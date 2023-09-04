use crate::db_notification::Notifier;
use crate::nino_constants::info;
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
use deno_core::anyhow::Error;
use http_types::Request;
use http_types::{Response, StatusCode};
use std::sync::Arc;

#[derive(Clone)]
pub struct DynamicManager {
    db: Arc<DBManager>,
    js_thread_count: usize,
    web_task_sx: Sender<nino_structures::JSTask>,
    web_task_rx: Receiver<nino_structures::JSTask>,
    notifier: Arc<Notifier>,
}

impl DynamicManager {
    pub fn new(
        db: Arc<DBManager>,
        js_thread_count: usize,
        notifier: Arc<Notifier>,
        db_subscribe: tokio::sync::broadcast::Receiver<nino_structures::NotificationMessage>,
    ) -> DynamicManager {
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

    async fn get_matching_path(&self, path: &str) -> Result<String, Error> {
        let query: String = format!(
            "SELECT name FROM {} WHERE name = $1",
            nino_constants::DYNAMICS_TABLE
        );
        let row = self.db.query_opt(&query, &[&path]).await?;
        match row {
            None => Err(Error::msg(format!(
                "dynamic '{}' does not exist in the database",
                path
            ))),
            Some(row) => {
                let path: String = row.get(0);
                Ok(path)
            }
        }
    }

    async fn get_matching_path_code(&self, path: &str) -> Result<String, Error> {
        let query: String = format!(
            "SELECT js FROM {} WHERE name = $1",
            nino_constants::DYNAMICS_TABLE
        );
        let row = self.db.query_opt(&query, &[&path]).await?;
        match row {
            None => Err(Error::msg(format!(
                "dynamic '{}' does not exist in the database",
                path
            ))),
            Some(row) => {
                let code: Vec<u8> = row.get(0);
                Ok(String::from_utf8(code)?)
            }
        }
    }

    // returns the longest matching path
    pub async fn get_module_js(&self, path: &str) -> Result<String, Error> {
        let query: String = format!(
            "SELECT js FROM {} WHERE name = $1",
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
                let js = String::from_utf8(js_bytes).unwrap();
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
        let js_module = self.get_matching_path_code(&request_info.name).await?;
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
        let js_module = self.get_matching_path(&request_info.name).await?;
        // default response
        let mut response = Response::new(200);
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

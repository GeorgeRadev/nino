use crate::db_notification::Notifier;
use crate::nino_functions;
use crate::{
    db::DBManager,
    nino_constants,
    nino_structures::{self, WebTask},
};
use async_channel::{Receiver, Sender};
use async_std::net::TcpStream;
use http_types::Request;
use http_types::{Mime, Response, StatusCode};
use std::str::FromStr;
use std::sync::Arc;

#[derive(Clone)]
pub struct DynamicManager {
    db: Arc<DBManager>,
    js_thread_count: u16,
    web_task_sx: Sender<Box<nino_structures::WebTask>>,
    web_task_rx: Receiver<Box<nino_structures::WebTask>>,
    notifier: Arc<Notifier>,
}

impl DynamicManager {
    pub fn new(
        db: Arc<DBManager>,
        js_thread_count: u16,
        notifier: Arc<Notifier>,
        db_subscribe: tokio::sync::broadcast::Receiver<nino_structures::Message>,
    ) -> DynamicManager {
        // web_task channel is used to send tasks to the js threads
        let (web_task_sx, web_task_rx) =
            async_channel::unbounded::<Box<nino_structures::WebTask>>();
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

    pub fn get_web_task_rx(&self) -> Receiver<Box<WebTask>> {
        self.web_task_rx.clone()
    }

    pub async fn invalidator(
        &self,
        mut db_subscribe: tokio::sync::broadcast::Receiver<nino_structures::Message>,
    ) {
        loop {
            match db_subscribe.recv().await {
                Err(error) => {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                }
                Ok(message) => {
                    println!("dymnamics got message: {}", message.json);
                    // send invalidation messages to the js threads
                    let web_task = Box::new(nino_structures::WebTask {
                        is_request: false,
                        js_module: None,
                        request: None,
                        stream: None,
                        is_invalidate: true,
                        message: message.json,
                    });
                    for _ in 0..self.js_thread_count {
                        if let Err(error) = self.web_task_sx.send(web_task.clone()).await {
                            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                        }
                    }
                }
            }
        }
    }

    // returns the longest matching path
    async fn get_matching_path(&self, path: &str) -> Option<String> {
        let query: String = format!(
            "SELECT name FROM {} WHERE name = $1",
            nino_constants::DYNAMICS_TABLE
        );
        match self.db.query(&query, &[&path]).await {
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                None
            }
            Ok(rows) => {
                for row in rows {
                    let path: String = row.get(0);
                    if !path.is_empty() {
                        return Some(path);
                    }
                }
                None
            }
        }
    }

    // returns the longest matching path
    pub async fn get_module_js(&self, path: &str) -> Option<String> {
        let query: String = format!(
            "SELECT js FROM {} WHERE name = $1",
            nino_constants::DYNAMICS_TABLE
        );
        let r = self.db.query_one(&query, &[&path]).await;
        match r {
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            }
            Ok(row) => {
                let js_bytes: Vec<u8> = row.get(0);
                let js = String::from_utf8(js_bytes).unwrap();
                if !js.is_empty() {
                    return Some(js);
                }
            }
        }
        None
    }

    pub async fn serve_dynamic(&self, path: &str, mut stream: Box<TcpStream>) -> bool {
        // look for matching path
        if let Some(js_module) = self.get_matching_path(path).await {
            let mut response = Response::new(StatusCode::Ok);
            response.set_content_type(Mime::from_str("application/javascript").unwrap());
            response.set_body(http_types::Body::from(js_module));
            match nino_functions::send_response_to_stream(stream.as_mut(), &mut response).await {
                Ok(_) => true,
                Err(error) => {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                    false
                }
            }
        } else {
            false
        }
    }

    pub async fn execute_dynamic(
        &self,
        path: &str,
        request: Request,
        stream: Box<TcpStream>,
    ) -> bool {
        // look for matching path
        if let Some(js_module) = self.get_matching_path(path).await {
            //send new task to the javascript threads
            let web_task = Box::new(nino_structures::WebTask {
                is_request: true,
                js_module: Some(js_module),
                request: Some(request),
                stream: Some(stream),
                is_invalidate: false,
                message: String::new(),
            });
            match self.web_task_sx.send(web_task).await {
                Ok(_) => true,
                Err(error) => {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                    //return error
                    false
                }
            }
        } else {
            false
        }
    }
}

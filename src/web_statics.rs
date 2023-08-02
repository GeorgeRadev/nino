use std::{str::FromStr, sync::Arc};

use crate::{db::DBManager, nino_constants, nino_functions, nino_structures};
use async_std::net::TcpStream;
use http_types::{Method, Mime, Request, Response, StatusCode};

#[derive(Clone)]
pub struct StaticManager {
    db: Arc<DBManager>,
}

impl StaticManager {
    pub fn new(
        db: Arc<DBManager>,
        db_subscribe: tokio::sync::broadcast::Receiver<nino_structures::Message>,
    ) -> StaticManager {
        let this = Self { db };
        let thizz = this.clone();
        tokio::spawn(async move {
            thizz.invalidator(db_subscribe).await;
        });
        this
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
                    println!("statics got message: {}", message.text);
                }
            }
        }
    }

    async fn get_static(&self, path: &str) -> Option<(String, Vec<u8>)> {
        let query: String = format!(
            "SELECT mime, content FROM {} WHERE name = $1",
            nino_constants::STATICS_TABLE
        );
        match self.db.query(&query, &[&path]).await {
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                None
            }
            Ok(rows) => {
                for row in rows {
                    let mime = row.get(0);
                    let content: Vec<u8> = row.get(1);
                    if !content.is_empty() {
                        return Some((mime, content));
                    }
                }
                None
            }
        }
    }

    pub async fn serve_static(&self, path: &str, request: Request, stream: Box<TcpStream>) -> bool {
        let method = request.method();
        if Method::Get != method {
            // handle only GET static requests
            return false;
        }
        let mut result = false;

        // look for exact path
        if let Some((mime, content)) = self.get_static(path).await {
            let mut response = Response::new(StatusCode::Ok);
            response.set_content_type(Mime::from_str(&mime).unwrap());
            response.set_body(http_types::Body::from(content));
            match nino_functions::send_response_to_stream(stream, &mut response).await {
                Ok(_) => {
                    result = true;
                }
                Err(error) => {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                }
            }
        }
        result
    }
}

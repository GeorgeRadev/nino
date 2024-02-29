use crate::{
    db::DBManager,
    nino_constants::{self, info},
    nino_functions, nino_structures,
    web_requests::RequestInfo,
};
use async_std::net::TcpStream;
use deno_runtime::deno_core::anyhow::Error;
use http_types::{Method, Request, Response, StatusCode};
use std::sync::Arc;

#[derive(Clone)]
pub struct StaticManager {
    db: Arc<DBManager>,
}

impl StaticManager {
    pub fn new(
        db: Arc<DBManager>,
        db_subscribe: tokio::sync::broadcast::Receiver<nino_structures::NotificationMessage>,
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
        mut db_subscribe: tokio::sync::broadcast::Receiver<nino_structures::NotificationMessage>,
    ) {
        loop {
            match db_subscribe.recv().await {
                Err(error) => {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                }
                Ok(message) => {
                    info!("MSG:statics: {}", message.text);
                }
            }
        }
    }

    async fn get_static(&self, path: &str) -> Result<Option<Vec<u8>>, Error> {
        let query: String = format!(
            "SELECT static_content FROM {} WHERE static_name = $1",
            nino_constants::STATICS_TABLE
        );
        match self.db.query_opt(&query, &[&path]).await {
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                Err(error)
            }
            Ok(row) => match row {
                None => Ok(None),
                Some(row) => {
                    let content: Vec<u8> = row.get(0);
                    Ok(Some(content))
                }
            },
        }
    }

    pub async fn serve_static(
        &self,
        request_info: RequestInfo,
        request: Request,
        stream: Box<TcpStream>,
    ) -> Result<(), Error> {
        let method = request.method();
        if Method::Get != method {
            // handle only GET static requests
            return Err(Error::msg("static requests handles only GET requests"));
        }
        // look for exact path
        if let Some(content) = self.get_static(&request_info.name).await? {
            let mut response = Response::new(StatusCode::Ok);
            response.set_content_type(request_info.mime);
            response.set_body(http_types::Body::from(content));
            nino_functions::send_response_to_stream(stream, &mut response).await?
        }
        Ok(())
    }
}

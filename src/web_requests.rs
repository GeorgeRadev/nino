use deno_runtime::deno_core::anyhow::Error;
use http_types::Mime;

use crate::{
    db::DBManager,
    db_notification,
    nino_constants::{self, USE_REQUEST_CACHE},
    nino_structures,
};
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, OnceLock, RwLock},
};

#[derive(Clone)]
pub struct RequestManager {
    db: Arc<DBManager>,
}

#[derive(Clone)]
pub struct RequestInfo {
    pub name: String,
    pub mime: Mime,
    pub redirect: bool,
    pub authorize: bool,
    pub dynamic: bool,
    pub execute: bool,
}

static REQUEST_CACHE: OnceLock<RwLock<HashMap<String, RequestInfo>>> = OnceLock::new();

impl RequestManager {
    pub fn new(
        db: Arc<DBManager>,
        db_subscribe: tokio::sync::broadcast::Receiver<nino_structures::NotificationMessage>,
    ) -> RequestManager {
        REQUEST_CACHE.get_or_init(|| RwLock::new(HashMap::new()));
        let this = Self { db };
        let thizz = this.clone();
        tokio::spawn(async move {
            thizz.reload_requests().await;
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
                    println!("MSG:requests: {}", message.text);
                    if message
                        .text
                        .starts_with(db_notification::NOTIFICATION_PREFIX_REQUESTS)
                    {
                        //reload the db aliases
                        self.reload_requests().await;
                    }
                }
            }
        }
    }

    async fn reload_requests(&self) {
        //reload the db aliases
        let query: String = format!(
            "SELECT request_path, request_name, request_mime_type, redirect_flag, authorize_flag, dynamic_flag, execute_flag FROM {}",
            nino_constants::REQUESTS_TABLE
        );
        match self.db.query(&query, &[]).await {
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            }
            Ok(rows) => {
                let mut map = REQUEST_CACHE.get().unwrap().write().unwrap();
                map.clear();
                for row in rows {
                    let path: String = row.get(0);
                    let name: String = row.get(1);
                    let mime_str: String = row.get(2);
                    let mime = Mime::from_str(&mime_str).unwrap();
                    let redirect: bool = row.get(3);
                    let authorize: bool = row.get(4);
                    let dynamic: bool = row.get(5);
                    let execute: bool = row.get(6);
                    map.insert(
                        path,
                        RequestInfo {
                            name,
                            mime,
                            redirect,
                            authorize,
                            dynamic,
                            execute,
                        },
                    );
                }
            }
        }
    }

    pub async fn get_request(&self, path: &String) -> Result<Option<RequestInfo>, Error> {
        if USE_REQUEST_CACHE {
            let map = REQUEST_CACHE.get().unwrap().read().unwrap();
            Ok(map.get(path).cloned())
        } else {
            let query: String = format!(
                "SELECT request_name, request_mime_type, redirect_flag, authorize_flag, dynamic_flag, execute_flag FROM {} WHERE path = $1",
                nino_constants::REQUESTS_TABLE
            );
            let result = self.db.query_opt(&query, &[&path]).await?;
            match result {
                None => Ok(None),
                Some(row) => {
                    let name: String = row.get(0);
                    let mime_str: String = row.get(1);
                    let mime = Mime::from_str(&mime_str).unwrap();
                    let redirect: bool = row.get(2);
                    let authorize: bool = row.get(3);
                    let dynamic: bool = row.get(4);
                    let execute: bool = row.get(5);
                    Ok(Some(RequestInfo {
                        name,
                        mime,
                        authorize,
                        dynamic,
                        redirect,
                        execute,
                    }))
                }
            }
        }
    }
}

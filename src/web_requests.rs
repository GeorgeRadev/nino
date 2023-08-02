use crate::{
    db::DBManager,
    db_notification,
    nino_constants::{self, USE_REQUEST_CACHE},
    nino_structures,
};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock, RwLock},
};

#[derive(Clone)]
pub struct RequestManager {
    db: Arc<DBManager>,
}

#[derive(Clone)]
pub struct RequestInfo {
    pub name: String,
    pub dynamic: bool,
    pub execute: bool,
    pub authorize: bool,
}

static REQUEST_CACHE: OnceLock<RwLock<HashMap<String, RequestInfo>>> = OnceLock::new();

impl RequestManager {
    pub fn new(
        db: Arc<DBManager>,
        db_subscribe: tokio::sync::broadcast::Receiver<nino_structures::Message>,
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
        mut db_subscribe: tokio::sync::broadcast::Receiver<nino_structures::Message>,
    ) {
        loop {
            match db_subscribe.recv().await {
                Err(error) => {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                }
                Ok(message) => {
                    // todo! innvalidate cache
                    println!("web got message: {}", message.text);
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
            "SELECT path, name, dynamic, execute, authorize FROM {}",
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
                    let dynamic: bool = row.get(2);
                    let execute: bool = row.get(3);
                    let authorize: bool = row.get(4);
                    map.insert(
                        path,
                        RequestInfo {
                            name,
                            dynamic,
                            execute,
                            authorize,
                        },
                    );
                }
            }
        }
    }

    pub async fn get_request(&self, path: &String) -> Option<RequestInfo> {
        if USE_REQUEST_CACHE {
            let map = REQUEST_CACHE.get().unwrap().read().unwrap();
            map.get(path).cloned()
        } else {
            let query: String = format!(
                "SELECT name, dynamic, execute, authorize FROM {} WHERE path = $1",
                nino_constants::REQUESTS_TABLE
            );
            match self.db.query_one(&query, &[&path]).await {
                Err(error) => {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                    None
                }
                Ok(row) => {
                    let name: String = row.get(0);
                    let dynamic: bool = row.get(1);
                    let execute: bool = row.get(2);
                    let authorize: bool = row.get(3);
                    Some(RequestInfo {
                        name,
                        dynamic,
                        execute,
                        authorize,
                    })
                }
            }
        }
    }
}

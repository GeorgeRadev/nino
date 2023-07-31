use tokio_postgres::Row;

use crate::db::DBManager;
use crate::{db_notification, nino_constants, nino_structures};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

// organize db connections in a map by alias to connections
// free all open aliases exept the first for keep it as pool

struct JSDBs {}

#[derive(Clone)]
struct JSDBInfo {
    pub db_type: String,
    pub connection_string: String,
}

static JSDB_ALIASES: OnceLock<RwLock<HashMap<String, JSDBInfo>>> = OnceLock::new();

// alias: Cell<HashMap<String, JSDBInfo>>,
#[derive(Clone)]
pub struct JSDBManager {
    db: Arc<DBManager>,
    alias_map: Arc<RwLock<HashMap<String, JSDBs>>>,
}

impl JSDBManager {
    /// Create DB Manager and connection pool
    pub fn new(
        db: Arc<DBManager>,
        db_subscribe: tokio::sync::broadcast::Receiver<nino_structures::Message>,
    ) -> JSDBManager {
        JSDB_ALIASES.get_or_init(|| RwLock::new(HashMap::new()));
        let map = HashMap::new();
        let alias_map = Arc::new(RwLock::new(map));
        let this = Self { db, alias_map };
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
                    println!("jsdb got message: {}", message.text);
                    if message
                        .text
                        .starts_with(db_notification::NOTIFICATION_PREFIX_DBNAME)
                    {
                        //reload the db aliases
                        let query: String = format!(
                            "SELECT db_name, db_type, db_connection_string FROM {}",
                            nino_constants::DATABASE_TABLE
                        );
                        match self.db.query(&query, &[]).await {
                            Ok(rows) => {
                                self.reload_databases(rows);
                            }
                            Err(error) => {
                                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                            }
                        }
                    }
                }
            }
        }
    }

    fn reload_databases(&self, rows: Vec<Row>) {
        let mut map = JSDB_ALIASES.get().unwrap().write().unwrap();
        for row in rows {
            let db_name: String = row.get(0);
            let db_type: String = row.get(1);
            let connection_string: String = row.get(2);
            map.insert(
                db_name,
                JSDBInfo {
                    db_type,
                    connection_string,
                },
            );
        }
    }

    pub fn cleanup(&self) {
        //todo! clean up all extra connections
    }

    pub fn get_connection(&self, db_name: String) -> Option<String> {
        let info = {
            let map = JSDB_ALIASES.get().unwrap().read().unwrap();
            match map.get(&db_name) {
                Some(info) => info.clone(),
                None => return None,
            }
        };
        if "postgres" == info.db_type {
        } else {
            // unsupported DB format
        }
        Some(String::new())
    }
}

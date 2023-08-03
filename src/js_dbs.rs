use crate::db::DBManager;
use crate::{db_notification, nino_constants, nino_structures};
use core::fmt;
use deno_core::anyhow::Error;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use tokio_postgres::types::ToSql;
use tokio_postgres::{Client, Config};

// organize db connections in a map by alias to connections
// free all open aliases exept the first for keep it as pool

#[derive(Clone)]
enum SupportedDatabases {
    Postgres,
    Unsupported(String),
}
impl fmt::Display for SupportedDatabases {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SupportedDatabases::Postgres => write!(f, "{}", nino_constants::DB_TYPE_POSTGRES),
            SupportedDatabases::Unsupported(db_type) => write!(f, "{}", db_type),
        }
    }
}

enum DatabaseConnection {
    Postgres(Client),
}

#[derive(Clone)]
struct JSDBInfo {
    pub db_type: SupportedDatabases,
    pub connection_string: String,
}

static JSDB_ALIASES: OnceLock<RwLock<HashMap<String, JSDBInfo>>> = OnceLock::new();

#[derive(Clone)]
pub struct JSDBManager {
    id: usize,
    db: Arc<DBManager>,
    pool_map: Arc<Mutex<HashMap<String, DatabaseConnection>>>,
}

impl JSDBManager {
    /// Create DB Manager and connection pool
    pub fn new(
        db: Arc<DBManager>,
        db_subscribe: tokio::sync::broadcast::Receiver<nino_structures::Message>,
    ) -> JSDBManager {
        static JSDB_THREAD_ID: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);

        JSDB_ALIASES.get_or_init(|| RwLock::new(HashMap::new()));
        let this = Self {
            id: JSDB_THREAD_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            db,
            pool_map: Arc::new(Mutex::new(HashMap::new())),
        };
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
                    // only one thread needs to invalidate the database aliases
                    if message
                        .text
                        .starts_with(db_notification::NOTIFICATION_PREFIX_DBNAME)
                    {
                        //reload the db aliases
                        if self.id == 0 {
                            self.reload_databases().await;
                        }
                    }
                }
            }
        }
    }

    async fn reload_databases(&self) {
        let query: String = format!(
            "SELECT db_alias, db_type, db_connection_string FROM {}",
            nino_constants::DATABASE_TABLE
        );
        match self.db.query(&query, &[]).await {
            Ok(rows) => {
                self.cleanup(true);
                let mut map = JSDB_ALIASES.get().unwrap().write().unwrap();
                map.clear();
                for row in rows {
                    let db_alias: String = row.get(0);
                    let db_type: String = row.get(1);
                    let connection_string: String = row.get(2);
                    map.insert(
                        db_alias,
                        JSDBInfo {
                            db_type: if nino_constants::DB_TYPE_POSTGRES == db_type {
                                SupportedDatabases::Postgres
                            } else {
                                SupportedDatabases::Unsupported(db_type)
                            },
                            connection_string,
                        },
                    );
                }
                map.insert(
                    String::from(nino_constants::MAIN_DB),
                    JSDBInfo {
                        db_type: SupportedDatabases::Postgres,
                        connection_string: self.db.get_connection_string(),
                    },
                );
            }
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            }
        }
    }

    pub fn cleanup(&self, all: bool) {
        if all {
            self.pool_map.lock().unwrap().clear();
        } else {
            let map = JSDB_ALIASES.get().unwrap().read().unwrap();
            self.pool_map
                .lock()
                .unwrap()
                .retain(move |key, _| map.contains_key(key));
        }
    }

    pub async fn create_db_connection(&self, db_alias: String) -> Result<String, Error> {
        if db_alias == nino_constants::MAIN_DB {
            // db alias is from the main database
            self.pool_add_postgres(db_alias, self.db.get_connection_string())
                .await
        } else {
            // db alias is another connection
            let info = {
                let map = JSDB_ALIASES.get().unwrap().read().unwrap();
                match map.get(&db_alias) {
                    Some(info) => info.clone(),
                    None => return Err(Error::msg(format!("db alias not found : {}", db_alias))),
                }
            };
            match info.db_type {
                SupportedDatabases::Postgres => {
                    self.pool_add_postgres(db_alias, info.connection_string)
                        .await
                }
                SupportedDatabases::Unsupported(db_type) => {
                    Err(Error::msg(format!("unsupported database type {}", db_type)))
                }
            }
        }
    }

    async fn pool_add_postgres(
        &self,
        db_alias: String,
        connection_string: String,
    ) -> Result<String, Error> {
        let name = {
            let pool = self.pool_map.lock().unwrap();
            if pool.contains_key(&db_alias) {
                format!("{}_{}", db_alias, pool.len())
            } else {
                db_alias
            }
        };
        // get db connection
        let config = connection_string.parse::<Config>().unwrap();
        let conn = config.connect(tokio_postgres::NoTls).await;
        match conn {
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                Err(Error::msg(format!("connection error {}", error)))
            }
            Ok((client, connection)) => {
                tokio::spawn(async move {
                    if let Err(error) = connection.await {
                        eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                    }
                });
                {
                    let mut pool = self.pool_map.lock().unwrap();
                    pool.insert(name.clone(), DatabaseConnection::Postgres(client));
                }
                Ok(name)
            }
        }
    }
    /*
    async fn alias_commit(conn: &mut DatabaseConnection) {
        match conn {
                DatabaseConnection::Postgres(_) => {
                    // todo
                }
            }
        }

        pub async fn commit_all(&self) {
            /*
            let mut map = self.pool_map.lock().unwrap();
            for conn in map.values_mut() {
                Self::alias_commit(conn).await;
            }
            */
        }

        pub async fn rollback_all(&self) {
            // todo
        }
        */

    fn query_convert_parameters(
        query: &Vec<String>,
        query_types: &Vec<i16>,
        i64vec: &mut Vec<Box<dyn ToSql + Sync>>,
    ) -> Result<(), Error> {
        let qlen = query.len();

        // try convert string to number
        for ix in 1..qlen {
            let val = query.get(ix).unwrap();
            if query_types[ix] == 0 {
                //boolean
                let b = val.eq_ignore_ascii_case("true") || val.eq("1");
                i64vec.push(Box::new(b));
            } else if query_types[ix] == 1 {
                //number
                match val.parse::<i64>() {
                    Ok(v) => {
                        i64vec.push(Box::new(v));
                    }
                    Err(_) => match val.parse::<f64>() {
                        Ok(v) => {
                            i64vec.push(Box::new(v));
                        }
                        Err(error) => {
                            return Err(Error::msg(format!(
                                "parameter {} `{}` is not number: {}",
                                ix, val, error
                            )));
                        }
                    },
                }
            } else {
                // use string value
                let b: Box<String> = Box::new(val.clone());
                i64vec.push(b);
            }
        }
        Ok(())
    }

    pub async fn execute_query(
        &self,
        db_alias: String,
        query: Vec<String>,
        query_types: Vec<i16>,
    ) -> Result<Vec<Vec<String>>, Error> {
        let qlen = query.len();
        if qlen < 1 {
            return Err(Error::msg("query is an empty array"));
        }
        let mut i64vec: Vec<Box<dyn ToSql + Sync>> = Vec::with_capacity(qlen);
        if let Err(error) = Self::query_convert_parameters(&query, &query_types, &mut i64vec) {
            return Err(error);
        }

        let mut params: Vec<&(dyn ToSql + Sync)> = Vec::with_capacity(qlen);
        for ix in 0..qlen - 1 {
            let sqlv: &(dyn ToSql + Sync) = (&i64vec[ix]).as_ref();
            params.push(sqlv);
        }

        let query = query[0].clone();

        let pool = self.pool_map.lock().unwrap();
        match pool.get(&db_alias) {
            None => {
                return Err(Error::msg(format!(
                    "db alias {} got disconnected",
                    db_alias
                )))
            }
            Some(db_connection) => match db_connection {
                DatabaseConnection::Postgres(client) => match client.query(&query, &params).await {
                    Err(error) => {
                        return Err(Error::msg(format!(
                            "ERROR: db alias {}: {}",
                            db_alias, error
                        )))
                    }
                    Ok(rows) => {
                        let mut result: Vec<Vec<String>> = Vec::new();
                        for row in rows {
                            let mut line: Vec<String> = Vec::new();
                            for ix in 0..row.len() {
                                let col_value: String = row.get(ix);
                                line.push(col_value);
                            }
                            result.push(line);
                        }
                        Ok(result)
                    }
                },
            },
        }
    }

    pub async fn execute_query_one(
        &self,
        db_alias: String,
        query: Vec<String>,
        query_types: Vec<i16>,
    ) -> Result<Vec<String>, Error> {
        let qlen = query.len();
        if qlen < 1 {
            return Err(Error::msg("query is an empty array"));
        }
        let mut i64vec: Vec<Box<dyn ToSql + Sync>> = Vec::with_capacity(qlen);
        if let Err(error) = Self::query_convert_parameters(&query, &query_types, &mut i64vec) {
            return Err(error);
        }
        let mut params: Vec<&(dyn ToSql + Sync)> = Vec::with_capacity(qlen);
        for ix in 0..qlen - 1 {
            let sqlv: &(dyn ToSql + Sync) = (&i64vec[ix]).as_ref();
            params.push(sqlv);
        }

        let query = query[0].clone();

        let pool = self.pool_map.lock().unwrap();
        match pool.get(&db_alias) {
            None => {
                return Err(Error::msg(format!(
                    "db alias {} got disconnected",
                    db_alias
                )))
            }
            Some(db_connection) => match db_connection {
                DatabaseConnection::Postgres(client) => {
                    match client.query_one(&query, &params).await {
                        Err(error) => {
                            return Err(Error::msg(format!(
                                "ERROR: db alias {}: {}",
                                db_alias, error
                            )))
                        }
                        Ok(row) => {
                            let mut result: Vec<String> = Vec::new();
                            for ix in 0..row.len() {
                                let col_value: String = row.get(ix);
                                result.push(col_value);
                            }

                            Ok(result)
                        }
                    }
                }
            },
        }
    }
}

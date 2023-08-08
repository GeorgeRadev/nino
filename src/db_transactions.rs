use crate::db::DBManager;
use crate::nino_constants;
use core::fmt;
use deno_core::anyhow::Error;
use postgres::Config;
use std::cell::RefCell;
use std::thread;

use postgres::types::ToSql;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::mpsc;

// organize db connections per js instance in a map by alias to connections
// free all open aliases exept the first for keep it as pool
// all channels and threads are because of Transaction<'_>
// Manager 1-*  Session  1-*  Transaction

static TRANSACTION_MANAGER: OnceLock<TransactionManager> = OnceLock::new();

#[derive(Clone)]
pub struct TransactionManager {
    id: usize,
    db: Arc<DBManager>,
}

impl TransactionManager {
    pub fn instance(id: usize, db: Arc<DBManager>) -> TransactionManager {
        TRANSACTION_MANAGER.get_or_init(|| TransactionManager { id, db });
        TRANSACTION_MANAGER.get().unwrap().clone()
    }

    pub fn register_transaction_session(&self) -> TransactionSession {
        let (request_in, request_out) = mpsc::channel::<TransactionSessionRequest>(16);
        let (response_in, response_out) = mpsc::channel::<TransactionSessionResponse>(16);

        let main_connection_string = self.db.get_connection_string().clone();
        if let Err(error) = thread::Builder::new()
            .name(format!("tx_session_{}", self.id))
            .spawn(move || {
                let mut tx = Transaction::new(main_connection_string, request_out, response_in);
                tx.start_session();
            })
        {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
        }
        TransactionSession {
            request_in: Arc::new(Mutex::new(request_in)),
            response_out: Arc::new(Mutex::new(response_out)),
        }
    }
}

#[derive(Clone)]
pub struct TransactionSession {
    request_in: Arc<Mutex<mpsc::Sender<TransactionSessionRequest>>>,
    response_out: Arc<Mutex<mpsc::Receiver<TransactionSessionResponse>>>,
}
impl TransactionSession {
    pub async fn reload_database_aliases(&mut self) -> Result<(), Error> {
        if let Err(error) = self
            .request_in
            .lock()
            .unwrap()
            .send(TransactionSessionRequest::ReloadDBAliases)
            .await
        {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            return Err(Error::msg(error));
        }
        match self.response_out.lock().unwrap().recv().await {
            None => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), "should not happen");
                Err(Error::msg("should not happen"))
            }
            Some(response) => match response {
                TransactionSessionResponse::Ok => Ok(()),
                TransactionSessionResponse::Error(msg) => Err(Error::msg(msg)),
                TransactionSessionResponse::UpsertResult(_) => panic!(),
                TransactionSessionResponse::QueryResult(_) => panic!(),
                TransactionSessionResponse::Transaction(_) => panic!(),
            },
        }
    }

    pub async fn create_db_connection(&mut self, db_alias: String) -> Result<String, Error> {
        if let Err(error) = self
            .request_in
            .lock()
            .unwrap()
            .send(TransactionSessionRequest::CreateTransaction(db_alias))
            .await
        {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            return Err(Error::msg(error));
        }
        match self.response_out.lock().unwrap().recv().await {
            None => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), "should not happen");
                Err(Error::msg("should not happen"))
            }
            Some(response) => match response {
                TransactionSessionResponse::Transaction(alias) => Ok(alias),
                TransactionSessionResponse::Error(msg) => Err(Error::msg(msg)),
                TransactionSessionResponse::UpsertResult(_) => panic!(),
                TransactionSessionResponse::QueryResult(_) => panic!(),
                TransactionSessionResponse::Ok => panic!(),
            },
        }
    }

    pub async fn close_all(&mut self, error: bool) -> Result<(), Error> {
        if let Err(error) = self
            .request_in
            .lock()
            .unwrap()
            .send(TransactionSessionRequest::CloseAll(error))
            .await
        {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            return Err(Error::msg(error));
        }
        match self.response_out.lock().unwrap().recv().await {
            None => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), "should not happen");
                Err(Error::msg("should not happen"))
            }
            Some(response) => match response {
                TransactionSessionResponse::Ok => Ok(()),
                TransactionSessionResponse::Error(msg) => Err(Error::msg(msg)),
                TransactionSessionResponse::UpsertResult(_) => panic!(),
                TransactionSessionResponse::QueryResult(_) => panic!(),
                TransactionSessionResponse::Transaction(_) => panic!(),
            },
        }
    }

    pub async fn query(
        &mut self,
        db_alias: String,
        query: Vec<String>,
        query_types: Vec<i16>,
        limit: usize,
    ) -> Result<QueryResult, Error> {
        let qlen = query.len();
        if qlen < 1 {
            return Err(Error::msg("query is an empty array"));
        }

        let query_data = QueryData {
            db_alias,
            params: query,
            param_types: query_types,
            limit,
        };

        if let Err(error) = self
            .request_in
            .lock()
            .unwrap()
            .send(TransactionSessionRequest::Query(query_data))
            .await
        {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            return Err(Error::msg(error));
        }
        match self.response_out.lock().unwrap().recv().await {
            None => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), "should not happen");
                Err(Error::msg("should not happen"))
            }
            Some(response) => match response {
                TransactionSessionResponse::QueryResult(result) => Ok(result),
                TransactionSessionResponse::Error(msg) => Err(Error::msg(msg)),
                TransactionSessionResponse::UpsertResult(_) => panic!(),
                TransactionSessionResponse::Transaction(_) => panic!(),
                TransactionSessionResponse::Ok => panic!(),
            },
        }
    }
}

#[derive(Clone)]
pub struct QueryData {
    db_alias: String,
    params: Vec<String>,
    // 0 - boolean
    // 1 - number
    // 2 - string
    // 3 - date
    param_types: Vec<i16>,
    limit: usize,
}

#[derive(Clone)]
pub struct QueryResult {
    pub rows: Vec<Vec<String>>,
    pub row_names: Vec<String>,
    pub row_types: Vec<String>,
}

#[derive(Clone)]
pub enum TransactionSessionRequest {
    ReloadDBAliases,
    CreateTransaction(String),
    CloseAll(bool),
    Query(QueryData),
    Upsert(QueryData),
}

#[derive(Clone)]
pub enum TransactionSessionResponse {
    Ok,
    Transaction(String),
    Error(String),
    QueryResult(QueryResult),
    UpsertResult(i32),
}

#[derive(Clone)]
pub enum TransactionRequest {
    Drop,
    Commit,
    Rollback,
    Query(QueryData),
    Upsert(QueryData),
}

#[derive(Clone)]
pub enum TransactionResponse {
    Ok,
    Error(String),
    QueryResult(QueryResult),
    UpsertResult(i32),
}

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

#[derive(Clone)]
struct DBAliasInfo {
    pub db_type: SupportedDatabases,
    pub connection_string: String,
}

enum DatabaseConnection {
    Postgres(TransactionPostgres),
}

pub struct Transaction {
    main_connection_string: String,
    request_out: mpsc::Receiver<TransactionSessionRequest>,
    response_in: mpsc::Sender<TransactionSessionResponse>,
    db_aliases: HashMap<String, DBAliasInfo>,
    db_pool: HashMap<String, RefCell<Box<dyn DBCommand>>>,
}

impl Transaction {
    fn new(
        main_connection_string: String,
        request_out: mpsc::Receiver<TransactionSessionRequest>,
        response_in: mpsc::Sender<TransactionSessionResponse>,
    ) -> Self {
        Self {
            main_connection_string,
            request_out,
            response_in,
            db_aliases: HashMap::with_capacity(32),
            db_pool: HashMap::with_capacity(32),
        }
    }

    fn start_session(&mut self) {
        // register main db
        self.db_aliases.insert(
            String::from(nino_constants::MAIN_DB),
            DBAliasInfo {
                db_type: SupportedDatabases::Postgres,
                connection_string: self.main_connection_string.clone(),
            },
        );
        // add main db to pool
        self.create_db_transaction(nino_constants::MAIN_DB.to_string());

        // ait for message and serve
        loop {
            if let Some(message) = self.request_out.blocking_recv() {
                // process transaction command
                let result = match message {
                    TransactionSessionRequest::ReloadDBAliases => {
                        self.reload_db_aliases();
                        TransactionSessionResponse::Ok
                    }
                    TransactionSessionRequest::CreateTransaction(db_alias) => {
                        match self.create_db_transaction(db_alias) {
                            Ok(alias) => TransactionSessionResponse::Transaction(alias),
                            Err(error) => TransactionSessionResponse::Error(format!(
                                "ERROR {}:{}:{}",
                                file!(),
                                line!(),
                                error
                            )),
                        }
                    }
                    TransactionSessionRequest::CloseAll(error) => match self.cleanup(error) {
                        Ok(_) => TransactionSessionResponse::Ok,
                        Err(error) => TransactionSessionResponse::Error(format!(
                            "ERROR {}:{}:{}",
                            file!(),
                            line!(),
                            error
                        )),
                    },
                    TransactionSessionRequest::Query(query_data) => {
                        match self.db_pool.get(&query_data.db_alias) {
                            None => TransactionSessionResponse::Error(format!(
                                "ERROR {}:{}: alias {} is missing",
                                file!(),
                                line!(),
                                query_data.db_alias
                            )),
                            Some(implementation) => {
                                let mut implementation = implementation.borrow_mut();
                                match implementation.query(query_data) {
                                    Ok(result) => TransactionSessionResponse::QueryResult(result),
                                    Err(error) => TransactionSessionResponse::Error(format!(
                                        "ERROR {}:{}:{}",
                                        file!(),
                                        line!(),
                                        error
                                    )),
                                }
                            }
                        }
                    }
                    TransactionSessionRequest::Upsert(_) => TransactionSessionResponse::Error(
                        format!("ERROR {}:{}:{}", file!(), line!(), "Not implemented yet"),
                    ),
                };
                // send response
                if let Err(error) = self.response_in.blocking_send(result) {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                    break;
                }
            }
        }
    }

    fn reload_db_aliases(&mut self) {
        match self.db_pool.get(nino_constants::MAIN_DB) {
            None => {
                eprintln!(
                    "ERROR {}:{}: db alias {} got disconnected",
                    file!(),
                    line!(),
                    nino_constants::MAIN_DB
                );
                return;
            }
            Some(db) => {
                let mut db = db.borrow_mut();

                let query: String = format!(
                    "SELECT db_alias, db_type, db_connection_string FROM {}",
                    nino_constants::DATABASE_TABLE
                );
                let query_data = QueryData {
                    db_alias: nino_constants::MAIN_DB.to_string(),
                    params: vec![query],
                    param_types: vec![2],
                    limit: 0,
                };
                match db.query(query_data) {
                    Ok(result) => {
                        for row in result.rows {
                            let db_alias: String = row.get(0).unwrap().clone();
                            let db_type: String = row.get(1).unwrap().clone();
                            let connection_string: String = row.get(2).unwrap().clone();
                            self.db_aliases.insert(
                                db_alias,
                                DBAliasInfo {
                                    db_type: if nino_constants::DB_TYPE_POSTGRES == db_type {
                                        // main is always  postgres type
                                        SupportedDatabases::Postgres
                                    } else {
                                        SupportedDatabases::Unsupported(db_type)
                                    },
                                    connection_string,
                                },
                            );
                        }
                    }
                    Err(error) => {
                        eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                    }
                }
            }
        }
    }

    fn cleanup(&mut self, error: bool) -> Result<(), Error> {
        self.db_pool.retain(move |_, db_session| {
            let mut tx = db_session.borrow_mut();
            if error {
                if let Err(error) = tx.rollback() {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                }
            } else {
                if let Err(error) = tx.commit() {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                }
            }
            // remove all
            true
        });
        self.create_db_transaction(nino_constants::MAIN_DB.to_string())?;
        Ok(())
    }

    fn create_db_transaction(&mut self, db_alias: String) -> Result<String, Error> {
        if db_alias == nino_constants::MAIN_DB {
            // db alias is from the main database
            self.db_pool_add_postgres(db_alias, self.main_connection_string.clone())
        } else {
            // db alias is another connection
            let info = {
                match self.db_aliases.get(&db_alias) {
                    Some(info) => info.clone(),
                    None => return Err(Error::msg(format!("db alias not found : {}", db_alias))),
                }
            };
            match info.db_type {
                SupportedDatabases::Postgres => {
                    self.db_pool_add_postgres(db_alias, info.connection_string)
                }
                SupportedDatabases::Unsupported(db_type) => {
                    Err(Error::msg(format!("unsupported database type {}", db_type)))
                }
            }
        }
    }

    fn db_pool_add_postgres(
        &mut self,
        db_alias: String,
        connection_string: String,
    ) -> Result<String, Error> {
        let name = {
            if self.db_pool.contains_key(&db_alias) {
                format!("{}_{}", db_alias, self.db_pool.len())
            } else {
                db_alias
            }
        };
        // create Database Transactional Session
        let db = TransactionPostgres::new(name.clone(), connection_string);
        self.db_pool
            .insert(name.clone(), RefCell::new(Box::new(db)));
        Ok(name)
    }
}

pub trait DBCommand {
    fn commit(&mut self) -> Result<(), Error>;
    fn rollback(&mut self) -> Result<(), Error>;
    fn query(&mut self, query_data: QueryData) -> Result<QueryResult, Error>;
}
struct TransactionPostgres {
    request_in: mpsc::Sender<TransactionRequest>,
    response_out: mpsc::Receiver<TransactionResponse>,
}

impl TransactionPostgres {
    fn new(alias: String, connection_string: String) -> Self {
        let (request_in, request_out) = mpsc::channel::<TransactionRequest>(16);
        let (response_in, response_out) = mpsc::channel::<TransactionResponse>(16);

        if let Err(error) = thread::Builder::new()
            .name(format!("tx_db_{}", alias))
            .spawn(move || {
                if let Err(error) =
                    Self::begin_db_transaction(connection_string, request_out, response_in)
                {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                }
            })
        {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
        }

        Self {
            request_in,
            response_out,
        }
    }

    fn begin_db_transaction(
        connection_string: String,
        mut request_out: mpsc::Receiver<TransactionRequest>,
        response_in: mpsc::Sender<TransactionResponse>,
    ) -> Result<(), Error> {
        let config = connection_string.parse::<Config>().unwrap();
        let mut conn = config.connect(tokio_postgres::NoTls)?;
        let mut tx = conn.transaction()?;

        loop {
            if let Some(message) = request_out.blocking_recv() {
                // process transaction command
                let response = match message {
                    TransactionRequest::Drop => return Ok(()),
                    TransactionRequest::Commit => {
                        let response = match tx.commit() {
                            Ok(_) => TransactionResponse::Ok,
                            Err(error) => TransactionResponse::Error(format!(
                                "ERROR {}:{}:{}",
                                file!(),
                                line!(),
                                error
                            )),
                        };
                        if let Err(error) = response_in.blocking_send(response) {
                            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                        }
                        return Ok(());
                    }
                    TransactionRequest::Rollback => {
                        let response = match tx.rollback() {
                            Ok(_) => TransactionResponse::Ok,
                            Err(error) => TransactionResponse::Error(format!(
                                "ERROR {}:{}:{}",
                                file!(),
                                line!(),
                                error
                            )),
                        };
                        if let Err(error) = response_in.blocking_send(response) {
                            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                        }
                        return Ok(());
                    }
                    TransactionRequest::Query(params) => {
                        let qlen = params.params.len();
                        let mut i64vec: Vec<Box<dyn ToSql + Sync>> = Vec::with_capacity(qlen);
                        match Self::query_convert_parameters(
                            &params.params,
                            &params.param_types,
                            &mut i64vec,
                        ) {
                            Err(error) => TransactionResponse::Error(format!(
                                "ERROR {}:{}:{}",
                                file!(),
                                line!(),
                                error
                            )),
                            Ok(_) => {
                                let mut qparams: Vec<&(dyn ToSql + Sync)> =
                                    Vec::with_capacity(qlen);
                                for ix in 0..qlen - 1 {
                                    let sqlv: &(dyn ToSql + Sync) = (&i64vec[ix]).as_ref();
                                    qparams.push(sqlv);
                                }
                                let query = params.params[0].clone();

                                match tx.query(&query, &qparams) {
                                    Err(error) => TransactionResponse::Error(format!(
                                        "ERROR {}:{}: db alias {} : {}",
                                        file!(),
                                        line!(),
                                        params.db_alias,
                                        error
                                    )),
                                    Ok(rows) => {
                                        let mut row_types: Vec<String> = Vec::new();
                                        let mut row_names: Vec<String> = Vec::new();
                                        let mut result: Vec<Vec<String>> = Vec::new();
                                        for row in rows {
                                            if row_types.len() == 0 {
                                                for column in row.columns() {
                                                    row_names.push(column.name().to_string());
                                                    let t = column.type_();
                                                    row_types.push(t.to_string());
                                                }
                                            }
                                            let mut line: Vec<String> = Vec::new();
                                            for ix in 0..row.len() {
                                                let col_value: String = row.get(ix);
                                                line.push(col_value);
                                            }
                                            result.push(line);
                                        }
                                        let query_result = QueryResult {
                                            rows: result,
                                            row_types,
                                            row_names,
                                        };
                                        TransactionResponse::QueryResult(query_result)
                                    }
                                }
                            }
                        }
                    }
                    TransactionRequest::Upsert(_) => {
                        todo!()
                    }
                };
                if let Err(error) = response_in.blocking_send(response) {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                }
            }
        }
    }

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
}

impl DBCommand for TransactionPostgres {
    fn commit(&mut self) -> Result<(), Error> {
        if let Err(error) = self.request_in.blocking_send(TransactionRequest::Commit) {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            return Err(Error::msg(error));
        }
        match self.response_out.blocking_recv() {
            None => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), "should not happen");
                Err(Error::msg("should not happen"))
            }
            Some(response) => match response {
                TransactionResponse::Ok => Ok(()),
                TransactionResponse::Error(msg) => Err(Error::msg(msg)),
                TransactionResponse::QueryResult(_) => todo!(),
                TransactionResponse::UpsertResult(_) => todo!(),
            },
        }
    }

    fn rollback(&mut self) -> Result<(), Error> {
        if let Err(error) = self.request_in.blocking_send(TransactionRequest::Rollback) {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            return Err(Error::msg(error));
        }
        match self.response_out.blocking_recv() {
            None => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), "should not happen");
                Err(Error::msg("should not happen"))
            }
            Some(response) => match response {
                TransactionResponse::Ok => Ok(()),
                TransactionResponse::Error(msg) => Err(Error::msg(msg)),
                TransactionResponse::QueryResult(_) => todo!(),
                TransactionResponse::UpsertResult(_) => todo!(),
            },
        }
    }

    fn query(&mut self, query_data: QueryData) -> Result<QueryResult, Error> {
        let qlen = query_data.params.len();
        if qlen < 1 {
            return Err(Error::msg("query is an empty array"));
        }
        if let Err(error) = self
            .request_in
            .blocking_send(TransactionRequest::Query(query_data))
        {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            return Err(Error::msg(error));
        }
        match self.response_out.blocking_recv() {
            None => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), "should not happen");
                Err(Error::msg("should not happen"))
            }
            Some(response) => match response {
                TransactionResponse::Ok => todo!(),
                TransactionResponse::Error(msg) => Err(Error::msg(msg)),
                TransactionResponse::QueryResult(result) => Ok(result),
                TransactionResponse::UpsertResult(_) => todo!(),
            },
        }
    }
}

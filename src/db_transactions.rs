use crate::nino_constants;
use async_channel::{Receiver, Sender};
use core::fmt;
use deno_core::anyhow::Error;
use std::collections::HashMap;
use std::thread;
use tokio_postgres::types::{to_sql_checked, IsNull, ToSql, Type};
use tokio_postgres::Config;

// organize db connections per js instance in a map by alias to connections
// free all open aliases exept the first for keep it as pool
// all channels and threads are because of Transaction<'_>
// Manager 1-*  Session  1-*  Transaction

#[derive(Clone, Debug)]
pub enum QueryParam {
    Null,
    Bool(bool),
    Number(i64),
    Float(f64),
    String(String),
    Date(String),
}

impl ToSql for QueryParam {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut tokio_postgres::types::private::BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>>
    where
        Self: Sized,
    {
        match self {
            QueryParam::Null => Ok(IsNull::Yes),
            QueryParam::Bool(v) => v.to_sql(ty, out),
            QueryParam::Number(v) => v.to_sql(ty, out),
            QueryParam::Float(v) => v.to_sql(ty, out),
            QueryParam::String(v) => v.to_sql(ty, out),
            QueryParam::Date(v) => v.to_sql(ty, out),
        }
    }

    fn accepts(_ty: &Type) -> bool
    where
        Self: Sized,
    {
        true
    }

    to_sql_checked!();
}

impl ToSql for Box<QueryParam> {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut tokio_postgres::types::private::BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>>
    where
        Self: Sized,
    {
        self.as_ref().to_sql(ty, out)
    }

    fn accepts(ty: &Type) -> bool {
        <&QueryParam as ToSql>::accepts(ty)
    }

    to_sql_checked!();
}

#[derive(Clone)]
pub struct QueryData {
    db_alias: String,
    query: String,
    params: Vec<QueryParam>,
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
    UpsertResult(u64),
}

#[derive(Clone)]
pub enum TransactionRequest {
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
    UpsertResult(u64),
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

pub struct TransactionManager {}

impl TransactionManager {
    // creates
    pub fn get_transaction_session(connection_string: String) -> TransactionSession {
        let main_connection_string = connection_string.clone();

        let (request_in, request_out) = async_channel::unbounded::<TransactionSessionRequest>();
        let (response_in, response_out) = async_channel::unbounded::<TransactionSessionResponse>();
        let builder = thread::Builder::new().name("TX Thread {}".to_string());
        if let Err(error) = builder.spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let mut tx = Transaction::new(main_connection_string, request_out, response_in);
            rt.block_on(tx.start_session());
        }) {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
        }
        TransactionSession {
            request_in,
            response_out,
        }
    }
}

pub struct TransactionSession {
    request_in: Sender<TransactionSessionRequest>,
    response_out: Receiver<TransactionSessionResponse>,
}
impl TransactionSession {
    pub fn reload_database_aliases(&mut self) -> Result<(), Error> {
        if let Err(error) = self
            .request_in
            .send_blocking(TransactionSessionRequest::ReloadDBAliases)
        {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            return Err(Error::msg(error));
        }
        match self.response_out.recv_blocking()? {
            TransactionSessionResponse::Ok => Ok(()),
            TransactionSessionResponse::Error(msg) => Err(Error::msg(msg)),
            TransactionSessionResponse::UpsertResult(_) => panic!(),
            TransactionSessionResponse::QueryResult(_) => panic!(),
            TransactionSessionResponse::Transaction(_) => panic!(),
        }
    }

    pub fn create_db_connection(&mut self, db_alias: String) -> Result<String, Error> {
        if let Err(error) = self
            .request_in
            .send_blocking(TransactionSessionRequest::CreateTransaction(db_alias))
        {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            return Err(Error::msg(error));
        }
        match self.response_out.recv_blocking()? {
            TransactionSessionResponse::Transaction(alias) => Ok(alias),
            TransactionSessionResponse::Error(msg) => Err(Error::msg(msg)),
            TransactionSessionResponse::UpsertResult(_) => panic!(),
            TransactionSessionResponse::QueryResult(_) => panic!(),
            TransactionSessionResponse::Ok => panic!(),
        }
    }

    pub fn close_all(&mut self, error: bool) -> Result<(), Error> {
        self.request_in
            .send_blocking(TransactionSessionRequest::CloseAll(error))?;
        match self.response_out.recv_blocking()? {
            TransactionSessionResponse::Ok => Ok(()),
            TransactionSessionResponse::Error(msg) => Err(Error::msg(msg)),
            TransactionSessionResponse::UpsertResult(_) => panic!(),
            TransactionSessionResponse::QueryResult(_) => panic!(),
            TransactionSessionResponse::Transaction(_) => panic!(),
        }
    }

    pub fn query(
        &mut self,
        db_alias: String,
        query: String,
        params: Vec<QueryParam>,
    ) -> Result<QueryResult, Error> {
        let query_data = QueryData {
            db_alias,
            query,
            params,
        };

        self.request_in
            .send_blocking(TransactionSessionRequest::Query(query_data))?;
        match self.response_out.recv_blocking()? {
            TransactionSessionResponse::QueryResult(result) => Ok(result),
            TransactionSessionResponse::Error(msg) => Err(Error::msg(msg)),
            TransactionSessionResponse::UpsertResult(_) => panic!(),
            TransactionSessionResponse::Transaction(_) => panic!(),
            TransactionSessionResponse::Ok => panic!(),
        }
    }

    pub fn upsert(
        &mut self,
        db_alias: String,
        query: String,
        params: Vec<QueryParam>,
    ) -> Result<u64, Error> {
        let query_data = QueryData {
            db_alias,
            query,
            params,
        };

        self.request_in
            .send_blocking(TransactionSessionRequest::Upsert(query_data))?;
        match self.response_out.recv_blocking()? {
            TransactionSessionResponse::UpsertResult(affected) => Ok(affected),
            TransactionSessionResponse::Error(msg) => Err(Error::msg(msg)),
            TransactionSessionResponse::QueryResult(_) => panic!(),
            TransactionSessionResponse::Transaction(_) => panic!(),
            TransactionSessionResponse::Ok => panic!(),
        }
    }
}

pub struct Transaction {
    main_connection_string: String,
    db_aliases: HashMap<String, DBAliasInfo>,
    db_pool: HashMap<String, TransactionInstance>,
    request_out: Receiver<TransactionSessionRequest>,
    response_in: Sender<TransactionSessionResponse>,
}

impl Transaction {
    fn new(
        main_connection_string: String,
        request_out: Receiver<TransactionSessionRequest>,
        response_in: Sender<TransactionSessionResponse>,
    ) -> Self {
        Self {
            main_connection_string,
            request_out,
            response_in,
            db_aliases: HashMap::with_capacity(32),
            db_pool: HashMap::with_capacity(32),
        }
    }

    async fn start_session(&mut self) {
        // register main db
        self.db_aliases.insert(
            String::from(nino_constants::MAIN_DB),
            DBAliasInfo {
                db_type: SupportedDatabases::Postgres,
                connection_string: self.main_connection_string.clone(),
            },
        );
        // add main db to pool
        if let Err(error) = self
            .create_db_transaction(nino_constants::MAIN_DB.to_string())
            .await
        {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
        }

        // wait for message and serve
        loop {
            eprintln!("------ loop Transaction");
            match self.request_out.recv().await {
                Ok(message) => {
                    // process transaction command
                    let result = match message {
                        TransactionSessionRequest::ReloadDBAliases => {
                            self.reload_db_aliases().await;
                            TransactionSessionResponse::Ok
                        }
                        TransactionSessionRequest::CreateTransaction(db_alias) => {
                            match self.create_db_transaction(db_alias).await {
                                Ok(alias) => TransactionSessionResponse::Transaction(alias),
                                Err(error) => TransactionSessionResponse::Error(format!(
                                    "ERROR {}:{}:{}",
                                    file!(),
                                    line!(),
                                    error
                                )),
                            }
                        }
                        TransactionSessionRequest::CloseAll(error) => {
                            match self.cleanup(error).await {
                                Ok(_) => TransactionSessionResponse::Ok,
                                Err(error) => TransactionSessionResponse::Error(format!(
                                    "ERROR {}:{}:{}",
                                    file!(),
                                    line!(),
                                    error
                                )),
                            }
                        }
                        TransactionSessionRequest::Query(query_data) => {
                            match self.db_pool.get_mut(&query_data.db_alias) {
                                None => TransactionSessionResponse::Error(format!(
                                    "ERROR {}:{}: alias {} is missing",
                                    file!(),
                                    line!(),
                                    query_data.db_alias
                                )),
                                Some(tx) => match tx {
                                    TransactionInstance::Postgres(tx) => {
                                        match tx.query(query_data).await {
                                            Ok(result) => {
                                                TransactionSessionResponse::QueryResult(result)
                                            }
                                            Err(error) => TransactionSessionResponse::Error(
                                                format!("ERROR {}:{}:{}", file!(), line!(), error),
                                            ),
                                        }
                                    }
                                },
                            }
                        }
                        TransactionSessionRequest::Upsert(query_data) => {
                            match self.db_pool.get_mut(&query_data.db_alias) {
                                None => TransactionSessionResponse::Error(format!(
                                    "ERROR {}:{}: alias {} is missing",
                                    file!(),
                                    line!(),
                                    query_data.db_alias
                                )),
                                Some(tx) => match tx {
                                    TransactionInstance::Postgres(tx) => {
                                        match tx.upsert(query_data).await {
                                            Ok(result) => {
                                                TransactionSessionResponse::UpsertResult(result)
                                            }
                                            Err(error) => TransactionSessionResponse::Error(
                                                format!("ERROR {}:{}:{}", file!(), line!(), error),
                                            ),
                                        }
                                    }
                                },
                            }
                        }
                    };
                    // send response
                    if let Err(error) = self.response_in.send(result).await {
                        eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                        break;
                    }
                }
                Err(error) => {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                    break;
                }
            }
        }
    }

    async fn reload_db_aliases(&mut self) {
        match self.db_pool.get_mut(nino_constants::MAIN_DB) {
            None => {
                eprintln!(
                    "ERROR {}:{}: db alias {} got disconnected",
                    file!(),
                    line!(),
                    nino_constants::MAIN_DB
                );
            }
            Some(tx) => match tx {
                TransactionInstance::Postgres(tx) => {
                    let query: String = format!(
                        "SELECT db_alias, db_type, db_connection_string FROM {}",
                        nino_constants::DATABASE_TABLE
                    );
                    let query_data = QueryData {
                        db_alias: nino_constants::MAIN_DB.to_string(),
                        query: query,
                        params: Vec::new(),
                    };
                    match tx.query(query_data).await {
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
            },
        }
    }

    async fn cleanup(&mut self, error: bool) -> Result<(), Error> {
        if error {
            for (_, tx) in self.db_pool.iter_mut() {
                match tx {
                    TransactionInstance::Postgres(tx) => {
                        if let Err(error) = tx.rollback().await {
                            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                        }
                    }
                }
            }
            self.db_pool.clear();
        } else {
            for (_, tx) in self.db_pool.iter_mut() {
                match tx {
                    TransactionInstance::Postgres(tx) => {
                        if let Err(error) = tx.commit().await {
                            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                        }
                    }
                }
            }
            self.db_pool.clear();
        }

        self.create_db_transaction(nino_constants::MAIN_DB.to_string())
            .await?;
        Ok(())
    }

    async fn create_db_transaction(&mut self, db_alias: String) -> Result<String, Error> {
        if db_alias == nino_constants::MAIN_DB {
            // db alias is from the main database
            self.db_pool_add_postgres(db_alias, self.main_connection_string.clone())
                .await
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
                        .await
                }
                SupportedDatabases::Unsupported(db_type) => {
                    Err(Error::msg(format!("unsupported database type {}", db_type)))
                }
            }
        }
    }

    async fn db_pool_add_postgres(
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
        let db = TransactionPostgres::new(connection_string).await;
        self.db_pool
            .insert(name.clone(), TransactionInstance::Postgres(db));
        Ok(name)
    }
}

enum TransactionInstance {
    Postgres(TransactionPostgres),
}

struct TransactionPostgres {
    request_in: Sender<TransactionRequest>,
    response_out: Receiver<TransactionResponse>,
}

impl TransactionPostgres {
    async fn new(connection_string: String) -> Self {
        let (request_in, request_out) = async_channel::unbounded::<TransactionRequest>();
        let (response_in, response_out) = async_channel::unbounded::<TransactionResponse>();

        tokio::spawn(async move {
            if let Err(error) =
                Self::begin_db_transaction(connection_string, request_out, response_in).await
            {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            }
        });
        Self {
            request_in,
            response_out,
        }
    }

    async fn commit(&mut self) -> Result<(), Error> {
        self.request_in.send(TransactionRequest::Commit).await?;
        match self.response_out.recv().await? {
            TransactionResponse::Ok => Ok(()),
            TransactionResponse::Error(msg) => Err(Error::msg(msg)),
            TransactionResponse::QueryResult(_) => todo!(),
            TransactionResponse::UpsertResult(_) => todo!(),
        }
    }

    async fn rollback(&mut self) -> Result<(), Error> {
        self.request_in.send(TransactionRequest::Rollback).await?;
        match self.response_out.recv().await? {
            TransactionResponse::Ok => Ok(()),
            TransactionResponse::Error(msg) => Err(Error::msg(msg)),
            TransactionResponse::QueryResult(_) => todo!(),
            TransactionResponse::UpsertResult(_) => todo!(),
        }
    }

    async fn query(&mut self, query_data: QueryData) -> Result<QueryResult, Error> {
        self.request_in
            .send(TransactionRequest::Query(query_data))
            .await?;
        match self.response_out.recv().await? {
            TransactionResponse::Ok => todo!(),
            TransactionResponse::Error(msg) => Err(Error::msg(msg)),
            TransactionResponse::QueryResult(result) => Ok(result),
            TransactionResponse::UpsertResult(_) => todo!(),
        }
    }

    async fn upsert(&mut self, query_data: QueryData) -> Result<u64, Error> {
        self.request_in
            .send(TransactionRequest::Upsert(query_data))
            .await?;
        match self.response_out.recv().await? {
            TransactionResponse::UpsertResult(affected) => Ok(affected),
            TransactionResponse::Error(msg) => Err(Error::msg(msg)),
            TransactionResponse::QueryResult(_) => panic!(),
            TransactionResponse::Ok => panic!(),
        }
    }

    async fn begin_db_transaction(
        connection_string: String,
        request_out: Receiver<TransactionRequest>,
        response_in: Sender<TransactionResponse>,
    ) -> Result<(), Error> {
        let config = connection_string.parse::<Config>().unwrap();
        let (mut conn, _) = config.connect(tokio_postgres::NoTls).await?;
        let tx = conn.transaction().await?;

        loop {
            eprintln!("------- loop TransactionPostgress");
            let response = match request_out.recv().await? {
                // process transaction command
                TransactionRequest::Commit => {
                    let response = match tx.commit().await {
                        Ok(_) => TransactionResponse::Ok,
                        Err(error) => {
                            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                            TransactionResponse::Error(format!(
                                "ERROR {}:{}:{}",
                                file!(),
                                line!(),
                                error
                            ))
                        }
                    };
                    response_in.send(response).await?;
                    return Ok(());
                }
                TransactionRequest::Rollback => {
                    let response = match tx.rollback().await {
                        Ok(_) => TransactionResponse::Ok,
                        Err(error) => {
                            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                            TransactionResponse::Error(format!(
                                "ERROR {}:{}:{}",
                                file!(),
                                line!(),
                                error
                            ))
                        }
                    };
                    response_in.send(response).await?;
                    return Ok(());
                }
                TransactionRequest::Query(query_data) => {
                    let dyn_vec: Vec<_> = query_data
                        .params
                        .iter()
                        .map(|v| v as &(dyn ToSql + Sync))
                        .collect();

                    match tx.query(&query_data.query, &dyn_vec).await {
                        Err(error) => {
                            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                            TransactionResponse::Error(format!(
                                "ERROR {}:{}: db alias {} : {}",
                                file!(),
                                line!(),
                                query_data.db_alias,
                                error
                            ))
                        }
                        Ok(rows) => {
                            let mut row_types: Vec<String> = Vec::new();
                            let mut row_names: Vec<String> = Vec::new();
                            let mut result: Vec<Vec<String>> = Vec::new();
                            for row in rows {
                                if row_types.is_empty() {
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
                TransactionRequest::Upsert(query_data) => {
                    // let qlen = query_data.params.len();
                    // let mut qparams: Vec<Box<(dyn ToSql + Sync)>> = Vec::with_capacity(qlen);
                    // for param in query_data.params {
                    //     qparams.push(Box::new(param) as Box<(dyn ToSql + Sync)>);
                    // }
                    let dyn_vec: Vec<_> = query_data
                        .params
                        .iter()
                        .map(|v| v as &(dyn ToSql + Sync))
                        .collect();

                    match tx.execute(&query_data.query, &dyn_vec).await {
                        Ok(affected) => TransactionResponse::UpsertResult(affected),
                        Err(error) => {
                            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                            TransactionResponse::Error(format!(
                                "ERROR {}:{}: db alias {} : {}",
                                file!(),
                                line!(),
                                query_data.db_alias,
                                error
                            ))
                        }
                    }
                }
            };
            response_in.send(response).await?;
        }
    }
}

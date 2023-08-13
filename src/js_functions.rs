use crate::db_notification::{self, Notifier};
use crate::db_transactions::{QueryParam, TransactionSession};
use crate::nino_structures;
use crate::web_dynamics::DynamicManager;
use crate::{db::DBManager, nino_functions};
use async_channel::Receiver;
use async_std::net::TcpStream;
use chrono::{DateTime, NaiveDateTime, Utc};
use deno_core::{anyhow::Error, op, Op, OpDecl, OpState};
use http_types::headers::CONTENT_TYPE;
use http_types::{Request, Response, StatusCode};
use std::sync::Arc;
use std::{cell::RefCell, rc::Rc};

pub fn get_javascript_ops() -> Vec<OpDecl> {
    vec![
        aop_sleep::DECL,
        op_begin_task::DECL,
        aop_end_task::DECL,
        op_get_request::DECL,
        op_set_response_status::DECL,
        op_set_response_header::DECL,
        aop_set_response_send_text::DECL,
        aop_set_response_send_json::DECL,
        aop_set_response_send_buf::DECL,
        op_get_invalidation_message::DECL,
        op_get_thread_id::DECL,
        aop_broadcast_message::DECL,
        op_get_module_invalidation_prefix::DECL,
        op_get_database_invalidation_prefix::DECL,
        aop_reload_database_aliases::DECL,
        aop_jsdb_get_connection_name::DECL,
        aop_jsdb_execute_query::DECL,
        aop_jsdb_execute_upsert::DECL,
    ]
}

pub struct JSContext {
    pub id: i16,
    pub db: Arc<DBManager>,
    pub dynamics: Arc<DynamicManager>,
    pub notifier: Arc<Notifier>,
    pub web_task_rx: Receiver<Box<nino_structures::JSTask>>,
    // response
    pub is_request: bool,
    pub module: String,
    pub request: Option<Request>,
    pub response: Option<Response>,
    pub stream: Option<Box<TcpStream>>,
    pub closed: bool,
    // invalidate
    pub is_invalidate: bool,
    pub message: String,
}

impl JSContext {
    pub fn clear(&mut self) {
        // response
        self.is_request = false;
        self.module = String::new();
        self.request = None;
        self.response = Some(Response::new(200));
        self.stream = None;
        self.closed = true;
        // invalidate
        self.is_invalidate = false;
        self.message = String::new();
    }
}

macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);

        // Find and cut the rest of the path
        match &name[..name.len() - 3].rfind(':') {
            Some(pos) => &name[pos + 1..name.len() - 3],
            None => &name[..name.len() - 3],
        }
    }};
}

// sync to async
// #[op]
// async fn op_async_task(state: Rc<RefCell<OpState>>) -> Result<(), Error> {
//     let future = tokio::task::spawn_blocking(move || {
//         // do some job here
//     });
//     future.await?;
//     Ok(())
// }

#[op]
fn op_begin_task(state: &mut OpState) -> Result<String, Error> {
    let context = state.borrow_mut::<JSContext>();
    let mut module = String::new();
    let result = context.web_task_rx.recv_blocking();
    match result {
        Ok(web_task) => {
            if web_task.is_request {
                // request
                context.is_request = true;
                if web_task.js_module.is_some() {
                    module = web_task.js_module.clone().unwrap();
                }
                context.module = module.clone();
                context.request = web_task.request;
                context.response = Some(Response::new(200));
                context.stream = web_task.stream;
                context.closed = false;
            } else if web_task.is_invalidate {
                // invalidate message
                context.is_invalidate = web_task.is_invalidate;
                context.message = web_task.message;
                context.closed = true;
            } else {
                // should not  get here
                panic!("should not get here")
            }
            // println!("new js task");
        }
        Err(error) => {
            context.closed = true;
            println!(
                "{}:{}:{} new js task ERROR: {}",
                function!(),
                line!(),
                context.id,
                error
            );
        }
    }
    Ok(module)
}

#[op]
async fn aop_end_task(op_state: Rc<RefCell<OpState>>, error: bool) -> Result<bool, Error> {
    {
        let mut state = op_state.borrow_mut();
        let tx = state.borrow_mut::<TransactionSession>();

        if let Err(error) = tx.close_all(error).await {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
        }
    }
    let stream;
    let mut response;
    {
        let mut state = op_state.borrow_mut();
        let context = state.borrow_mut::<JSContext>();

        if context.closed {
            //task already closed
            return Ok(false);
        }

        context.clear();

        stream = context.stream.take().unwrap();
        response = context.response.take().unwrap();
    }

    if let Err(error) = nino_functions::send_response_to_stream(stream, &mut response).await {
        eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
    }
    Ok(true)
}

#[derive(deno_core::serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequest {
    url: http_types::Url,
    method: String,
    original_url: String,
    host: String,
    path: String,
    query: String,
}

#[op]
fn op_get_request(state: &mut OpState) -> Result<HttpRequest, Error> {
    let context = state.borrow_mut::<JSContext>();
    if !context.is_request {
        return Err(Error::msg("task is not a request"));
    }
    let request = context.request.as_mut().unwrap();
    let url = request.url();
    let url_str = url.to_string();

    let request = HttpRequest {
        url: url.clone(),
        method: request.method().to_string(),
        original_url: url_str,
        host: String::from(url.host_str().unwrap_or("")),
        path: String::from(url.path()),
        query: String::from(url.query().unwrap_or("")),
    };
    //deno_core::serde_json::to_string(&request).unwrap()
    Ok(request)
}

#[op]
fn op_set_response_status(state: &mut OpState, status: u16) -> Result<(), Error> {
    let context = state.borrow_mut::<JSContext>();
    let status = StatusCode::try_from(status).unwrap();
    context.response.as_mut().unwrap().set_status(status);
    Ok(())
}

#[op]
fn op_set_response_header(state: &mut OpState, key: String, value: String) -> Result<(), Error> {
    let context = state.borrow_mut::<JSContext>();
    let res = context.response.as_mut().unwrap();
    res.remove_header(&*key);
    res.append_header(&*key, &*value);
    Ok(())
}

#[op]
async fn aop_set_response_send_text(
    op_state: Rc<RefCell<OpState>>,
    body: String,
) -> Result<(), Error> {
    aop_set_response_send(op_state, "text/html;charset=UTF-8", body).await
}

#[op]
async fn aop_set_response_send_json(
    op_state: Rc<RefCell<OpState>>,
    body: String,
) -> Result<(), Error> {
    aop_set_response_send(op_state, "application/json;charset=UTF-8", body).await
}

async fn aop_set_response_send(
    op_state: Rc<RefCell<OpState>>,
    mime: &str,
    body: String,
) -> Result<(), Error> {
    let stream;
    let mut response;
    {
        let mut state = op_state.borrow_mut();
        let context = state.borrow_mut::<JSContext>();
        if context.closed {
            //task already closed
            return Ok(());
        }
        if !context.is_request {
            return Err(Error::msg("task is not a request"));
        }

        stream = context.stream.take().unwrap();
        response = context.response.take().unwrap();
        context.clear();
    }

    let has_no_type = response.header(CONTENT_TYPE).is_none();
    if has_no_type {
        response.insert_header(CONTENT_TYPE, mime);
    }
    response.set_body(body);

    if let Err(error) = nino_functions::send_response_to_stream(stream, &mut response).await {
        eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
    };
    Ok(())
}

#[op]
async fn aop_set_response_send_buf(
    op_state: Rc<RefCell<OpState>>,
    buffer: Vec<u8>,
) -> Result<(), Error> {
    let stream;
    let mut response;
    {
        let mut state = op_state.borrow_mut();
        let context = state.borrow_mut::<JSContext>();
        if context.closed {
            //task already closed
            return Ok(());
        }
        if !context.is_request {
            return Err(Error::msg("task is not a request"));
        }

        stream = context.stream.take().unwrap();
        response = context.response.take().unwrap();
        context.clear();
    }

    let has_no_type = response.header(CONTENT_TYPE).is_none();
    response.set_body(buffer);
    if has_no_type {
        response.insert_header(CONTENT_TYPE, "text/html;charset=UTF-8");
    }

    if let Err(error) = nino_functions::send_response_to_stream(stream, &mut response).await {
        eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
    };
    Ok(())
}

#[op]
async fn aop_sleep(op_state: Rc<RefCell<OpState>>, millis: u64) -> Result<(), Error> {
    // Future must be Poll::Pending on first call
    let v;
    {
        let state = op_state.borrow();
        let context = state.borrow::<JSContext>();
        v = context.id;
    }
    println!("{} waiting", v);
    tokio::time::sleep(std::time::Duration::from_millis(millis)).await;
    Ok(())
}

#[op]
fn op_get_invalidation_message(state: &mut OpState) -> String {
    let context = state.borrow_mut::<JSContext>();
    if context.is_invalidate {
        return context.message.clone();
    }
    String::new()
}

#[op]
fn op_get_thread_id(state: &mut OpState) -> i16 {
    let context = state.borrow_mut::<JSContext>();
    context.id
}

#[op]
async fn aop_broadcast_message(
    op_state: Rc<RefCell<OpState>>,
    message: String,
) -> Result<bool, Error> {
    let notifier = {
        let mut state = op_state.borrow_mut();
        let context = state.borrow_mut::<JSContext>();
        context.notifier.clone()
    };
    match notifier.notify(message).await {
        Ok(_) => Ok(true),
        Err(error) => Err(Error::msg(error)),
    }
}

#[op]
fn op_get_module_invalidation_prefix() -> String {
    String::from(db_notification::NOTIFICATION_PREFIX_DYNAMICS)
}

#[op]
fn op_get_database_invalidation_prefix() -> String {
    String::from(db_notification::NOTIFICATION_PREFIX_DBNAME)
}

#[op]
async fn aop_reload_database_aliases(op_state: Rc<RefCell<OpState>>) -> Result<(), Error> {
    let mut state = op_state.borrow_mut();
    let tx = state.borrow_mut::<TransactionSession>();
    tx.reload_database_aliases().await
}

#[op]
async fn aop_jsdb_get_connection_name(
    op_state: Rc<RefCell<OpState>>,
    db_alias: String,
) -> Result<String, Error> {
    let mut state = op_state.borrow_mut();
    let tx = state.borrow_mut::<TransactionSession>();
    tx.create_db_connection(db_alias).await
}

#[derive(deno_core::serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub rows: Vec<Vec<String>>,
    pub row_names: Vec<String>,
    pub row_types: Vec<String>,
}

fn query_types_to_params(
    query: Vec<String>,
    query_types: Vec<i16>,
) -> Result<(String, Vec<QueryParam>), Error> {
    let qlen = query.len();
    {
        if qlen == 0 {
            return Err(Error::msg("query must contain atleast the query"));
        }
        let tlen = query_types.len();
        if qlen != tlen {
            return Err(Error::msg("query values and query types must be same size"));
        }
    }

    let mut query_params: Vec<QueryParam> = Vec::with_capacity(qlen);
    // JS types
    // 0 - NULL
    // 1 - Boolean
    // 2 - Number
    // 3 - String
    // 4 - Date
    for ix in 1..qlen - 1 {
        let v = query.get(ix).unwrap();
        let t = query_types[ix];
        if t == 0 {
            //NULL
            query_params.push(QueryParam::Null);
        } else if t == 1 {
            //boolean
            let b = v.eq_ignore_ascii_case("true") || v.eq("1");
            query_params.push(QueryParam::Bool(b));
        } else if t == 2 {
            //number
            match v.parse::<i64>() {
                Ok(v) => {
                    query_params.push(QueryParam::Number(v));
                }
                Err(_) => match v.parse::<f64>() {
                    Ok(v) => {
                        query_params.push(QueryParam::Float(v));
                    }
                    Err(e) => {
                        return Err(Error::msg(format!(
                            "parameter {} `{}` is not number: {}",
                            ix, v, e
                        )));
                    }
                },
            }
        } else if query_types[ix] == 4 {
            //date
            match v.parse::<i64>() {
                Ok(v) => {
                    let secs = v / 1000;
                    let ns = (v % 1000) * 1_000_000;
                    let ndt = NaiveDateTime::from_timestamp_opt(secs, ns as u32).unwrap();
                    let dt = DateTime::<Utc>::from_utc(ndt, Utc);
                    let v = dt.to_rfc3339();
                    query_params.push(QueryParam::Date(v));
                }
                Err(error) => {
                    return Err(Error::msg(format!(
                        "parameter {} `{}` is not UTC miliseconds: {}",
                        ix, v, error
                    )));
                }
            };
        } else {
            // use string value
            query_params.push(QueryParam::String(v.clone()));
        }
    }
    Ok((query[0].clone(), query_params))
}

#[op]
async fn aop_jsdb_execute_query(
    op_state: Rc<RefCell<OpState>>,
    db_alias: String,
    query: Vec<String>,
    query_types: Vec<i16>,
) -> Result<QueryResult, Error> {
    let (query, params) = query_types_to_params(query, query_types)?;
    let mut state = op_state.borrow_mut();
    let tx = state.borrow_mut::<TransactionSession>();
    let result = tx.query(db_alias, query, params).await?;
    Ok(QueryResult {
        rows: result.rows,
        row_names: result.row_names,
        row_types: result.row_types,
    })
}

#[op]
async fn aop_jsdb_execute_upsert(
    op_state: Rc<RefCell<OpState>>,
    db_alias: String,
    query: Vec<String>,
    query_types: Vec<i16>,
) -> Result<u64, Error> {
    let (query, params) = query_types_to_params(query, query_types)?;
    let mut state = op_state.borrow_mut();
    let tx = state.borrow_mut::<TransactionSession>();
    tx.upsert(db_alias, query, params).await
}

use crate::db_notification::{self, Notifier};
use crate::db_transactions::{QueryParam, TransactionSession};
use crate::nino_constants::{self, info};
use crate::nino_functions;
use crate::nino_structures::JSTask;
use crate::web_dynamics::DynamicManager;
use async_channel::Receiver;
use deno_core::{anyhow::Error, op, Op, OpDecl, OpState};
use http_types::StatusCode;
use hyper::Client;
use hyper_tls::HttpsConnector;
use std::collections::HashMap;
use std::sync::Arc;
use std::{cell::RefCell, rc::Rc};

pub fn get_javascript_ops() -> Vec<OpDecl> {
    vec![
        aop_sleep::DECL,
        op_begin_task::DECL,
        aop_end_task::DECL,
        op_get_request::DECL,
        op_get_request_body::DECL,
        op_set_response_status::DECL,
        op_set_response_header::DECL,
        aop_set_response_send_text::DECL,
        aop_set_response_send_buf::DECL,
        op_get_invalidation_message::DECL,
        op_get_thread_id::DECL,
        op_broadcast_message::DECL,
        aop_broadcast_message::DECL,
        op_get_module_invalidation_prefix::DECL,
        op_get_database_invalidation_prefix::DECL,
        op_reload_database_aliases::DECL,
        op_tx_end::DECL,
        op_tx_get_connection_name::DECL,
        op_tx_execute_query::DECL,
        op_tx_execute_upsert::DECL,
        op_get_user_jwt::DECL,
        aop_fetch::DECL,
    ]
}

pub struct JSContext {
    pub id: i16,
    pub dynamics: Arc<DynamicManager>,
    pub notifier: Arc<Notifier>,
    pub web_task_rx: Receiver<JSTask>,
    // close request will have a None Task
    pub task: Option<JSTask>,
    // collect broadcast messages to be send after commit
    pub broadcast_messages: Vec<String>,
}

impl JSContext {
    pub fn clear(&mut self) {
        self.task = None;
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
    let mut module = String::new();
    let context = state.borrow_mut::<JSContext>();
    let result = context.web_task_rx.recv_blocking();
    // return execution module or empty string if not a Servlet
    match result {
        Ok(task) => {
            context.task = Some(task);
            if let Some(task) = &context.task {
                match task {
                    JSTask::Message(_) => {
                        // nothing to do here
                    }
                    JSTask::Servlet(request) => {
                        module = request.js_module.clone();
                    }
                }
            }
            // info!("new js task");
        }
        Err(error) => {
            context.clear();
            // should happen only when terminating program
            info!("OK {}:{}:{} {}", function!(), line!(), context.id, error);
        }
    }
    Ok(module)
}

#[op]
fn op_tx_end(state: &mut OpState, commit: bool) -> Result<(), Error> {
    let tx = state.borrow_mut::<TransactionSession>();
    if let Err(error) = tx.close_all(commit) {
        eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
    }
    Ok(())
}

#[op]
async fn aop_end_task(op_state: Rc<RefCell<OpState>>) -> Result<bool, Error> {
    let stream;
    let mut response;
    {
        let mut state = op_state.borrow_mut();
        let context = state.borrow_mut::<JSContext>();

        if context.task.is_none() {
            //task already closed
            return Ok(false);
        }

        match context.task.take().unwrap() {
            JSTask::Message(_) => {
                // nothing to do here
                return Ok(false);
            }
            JSTask::Servlet(request) => {
                stream = request.stream;
                response = request.response;
                context.clear();
            }
        }
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
    original_url: String,
    method: String,
    host: String,
    path: String,
    query: String,
    parameters: HashMap<String, Vec<String>>,
    user: String,
}

#[op]
fn op_get_request(state: &mut OpState) -> Result<HttpRequest, Error> {
    let context = state.borrow_mut::<JSContext>();

    if let Some(task) = &context.task {
        match task {
            JSTask::Servlet(servlet) => {
                let url = servlet.request.url();
                let url_str = url.to_string();
                let query = String::from(url.query().unwrap_or(""));
                let mut parameters: HashMap<String, Vec<String>> = HashMap::new();
                for (key, value) in url.query_pairs() {
                    let key = key.to_string();
                    let value = value.to_string();
                    match parameters.get_mut(&key) {
                        None => {
                            let mut vec: Vec<String> = Vec::with_capacity(2);
                            vec.push(value);
                            parameters.insert(key, vec);
                        }
                        Some(vec) => {
                            vec.push(value);
                        }
                    }
                }

                let request = HttpRequest {
                    url: url.clone(),
                    method: servlet.request.method().to_string(),
                    original_url: url_str,
                    host: String::from(url.host_str().unwrap_or("")),
                    path: String::from(url.path()),
                    query,
                    parameters,
                    user: servlet.user.clone(),
                };
                //deno_core::serde_json::to_string(&request).unwrap()
                Ok(request)
            }
            JSTask::Message(_) => Err(Error::msg("task is not a request")),
        }
    } else {
        Err(Error::msg("no current task"))
    }
}

#[op]
fn op_set_response_status(state: &mut OpState, status: u16) -> Result<(), Error> {
    let context = state.borrow_mut::<JSContext>();

    if let Some(task) = &mut context.task {
        match task {
            JSTask::Servlet(servlet) => {
                let status = StatusCode::try_from(status).unwrap();
                servlet.response.set_status(status);
                Ok(())
            }
            JSTask::Message(_) => Err(Error::msg("task is not a request")),
        }
    } else {
        Err(Error::msg("no current task"))
    }
}

#[op]
fn op_set_response_header(state: &mut OpState, key: String, value: String) -> Result<(), Error> {
    let context = state.borrow_mut::<JSContext>();

    if let Some(task) = &mut context.task {
        match task {
            JSTask::Servlet(servlet) => {
                let response = &mut servlet.response;
                response.remove_header(&*key);
                response.append_header(&*key, &*value);
                Ok(())
            }
            JSTask::Message(_) => Err(Error::msg("task is not a request")),
        }
    } else {
        Err(Error::msg("no current task"))
    }
}

#[op]
async fn aop_set_response_send_text(
    op_state: Rc<RefCell<OpState>>,
    body: String,
) -> Result<(), Error> {
    aop_set_response_send(op_state, body).await
}

async fn aop_set_response_send(op_state: Rc<RefCell<OpState>>, body: String) -> Result<(), Error> {
    let stream;
    let mut response;
    {
        let mut state = op_state.borrow_mut();
        let context = state.borrow_mut::<JSContext>();

        if context.task.is_none() {
            //task already closed
            return Ok(());
        }

        match context.task.take().unwrap() {
            JSTask::Servlet(servlet) => {
                stream = servlet.stream;
                response = servlet.response;
            }
            JSTask::Message(_) => {
                return Err(Error::msg("task is not a request"));
            }
        }
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

        if context.task.is_none() {
            //task already closed
            return Ok(());
        }

        match context.task.take().unwrap() {
            JSTask::Servlet(servlet) => {
                stream = servlet.stream;
                response = servlet.response;
            }
            JSTask::Message(_) => {
                return Err(Error::msg("task is not a request"));
            }
        }
    }

    response.set_body(buffer);

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

    if context.task.is_some() {
        match context.task.as_mut().unwrap() {
            JSTask::Message(message) => message.clone(),
            JSTask::Servlet(_) => String::new(),
        }
    } else {
        String::new()
    }
}

#[op]
fn op_get_thread_id(state: &mut OpState) -> i16 {
    let context = state.borrow_mut::<JSContext>();
    context.id
}

#[op]
fn op_broadcast_message(state: &mut OpState, message: String) {
    let context = state.borrow_mut::<JSContext>();
    context.broadcast_messages.push(message);
}

#[op]
async fn aop_broadcast_message(op_state: Rc<RefCell<OpState>>, commit: bool) {
    let notifier;
    let mut messages: Vec<String> = Vec::with_capacity(8);
    {
        let mut state = op_state.borrow_mut();
        let context = state.borrow_mut::<JSContext>();
        notifier = context.notifier.clone();
        if commit {
            messages.append(&mut context.broadcast_messages);
        } else {
            context.broadcast_messages.clear();
        }
    };
    if !commit {
        for message in messages {
            if let Err(error) = notifier.notify(message).await {
                eprintln!("ERROR {}:{}:{}", function!(), line!(), error);
            }
        }
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
    for (ix, value) in query.iter().enumerate().take(qlen).skip(1) {
        let t = query_types[ix];
        if t == 0 {
            //NULL
            query_params.push(QueryParam::Null);
        } else if t == 1 {
            //boolean
            let b = value.eq_ignore_ascii_case("true") || value.eq("1");
            query_params.push(QueryParam::Bool(b));
        } else if t == 2 {
            //number
            match value.parse::<i64>() {
                Ok(v) => {
                    query_params.push(QueryParam::Number(v));
                }
                Err(_) => match value.parse::<f64>() {
                    Ok(v) => {
                        query_params.push(QueryParam::Float(v));
                    }
                    Err(e) => {
                        return Err(Error::msg(format!(
                            "parameter {} `{}` is not number: {}",
                            ix, value, e
                        )));
                    }
                },
            }
        } else if query_types[ix] == 4 {
            //date
            match value.parse::<i64>() {
                Ok(v) => {
                    let secs = v / 1000;
                    let ns = (v % 1000) * 1_000_000;
                    let ndt = chrono::NaiveDateTime::from_timestamp_opt(secs, ns as u32).unwrap();
                    let dt = chrono::DateTime::<chrono::Utc>::from_utc(ndt, chrono::Utc);
                    let v = dt.to_rfc3339();
                    query_params.push(QueryParam::Date(v));
                }
                Err(error) => {
                    return Err(Error::msg(format!(
                        "parameter {} `{}` is not UTC miliseconds: {}",
                        ix, value, error
                    )));
                }
            };
        } else {
            // use string value
            query_params.push(QueryParam::String(value.clone()));
        }
    }
    Ok((query[0].clone(), query_params))
}

#[op]
fn op_reload_database_aliases(state: &mut OpState) -> Result<(), Error> {
    let tx = state.borrow_mut::<TransactionSession>();
    tx.reload_database_aliases()
}

#[op]
fn op_tx_get_connection_name(state: &mut OpState, db_alias: String) -> Result<String, Error> {
    let tx = state.borrow_mut::<TransactionSession>();
    tx.create_transaction(db_alias)
}

#[op]
fn op_tx_execute_query(
    state: &mut OpState,
    db_alias: String,
    query: Vec<String>,
    query_types: Vec<i16>,
) -> Result<QueryResult, Error> {
    let (query, params) = query_types_to_params(query, query_types)?;
    let tx = state.borrow_mut::<TransactionSession>();

    let result = tx.query(db_alias, query, params)?;
    Ok(QueryResult {
        rows: result.rows,
        row_names: result.row_names,
        row_types: result.row_types,
    })
}

#[op]
fn op_tx_execute_upsert(
    state: &mut OpState,
    db_alias: String,
    query: Vec<String>,
    query_types: Vec<i16>,
) -> Result<u64, Error> {
    let (query, params) = query_types_to_params(query, query_types)?;
    let tx = state.borrow_mut::<TransactionSession>();
    tx.upsert(db_alias, query, params)
}

#[op]
fn op_get_request_body(state: &mut OpState) -> Result<String, Error> {
    let context = state.borrow_mut::<JSContext>();

    if context.task.is_some() {
        if let Some(JSTask::Servlet(servlet)) = context.task.as_mut() {
            Ok(servlet.body.clone())
        } else {
            Err(Error::msg("task is not a request"))
        }
    } else {
        Err(Error::msg("no current task"))
    }
}

#[op]
fn op_get_user_jwt(_state: &mut OpState, user: String) -> Result<String, Error> {
    let mut map: HashMap<String, String> = HashMap::new();
    map.insert(nino_constants::JWT_USER.to_string(), user);
    nino_functions::jwt_from_map(nino_constants::PROGRAM_NAME, map)
}

#[op]
async fn aop_fetch(
    _op_state: Rc<RefCell<OpState>>,
    url: String,
    timeout: i64,
    method: String,
    headers: HashMap<String, String>,
    body: String,
) -> Result<String, Error> {
    let uri: hyper::Uri = url.parse()?;
    let https = uri.scheme_str() == Some("https");
    // Build out our request
    let mut request_builder = hyper::Request::builder();
    request_builder = request_builder.uri(uri);
    request_builder = if method.is_empty() {
        request_builder.method(hyper::Method::GET)
    } else {
        request_builder.method(hyper::Method::from_bytes(method.as_bytes())?)
    };
    for (key, value) in headers.iter() {
        request_builder = request_builder.header(key, value);
    }
    let request = request_builder.body(hyper::Body::from(body))?;

    let response_future = if https {
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, hyper::Body>(https);
        client.request(request)
    } else {
        let client = Client::new();
        client.request(request)
    };

    match tokio::time::timeout(
        tokio::time::Duration::from_millis(timeout as u64),
        response_future,
    )
    .await
    {
        Err(_) => Err(Error::msg("Connection Timeout")),
        Ok(Err(e)) => Err(e.into()),
        Ok(Ok(response)) => {
            let body_bytes = hyper::body::to_bytes(response.into_body()).await?;
            let body = String::from_utf8(body_bytes.to_vec())?;
            Ok(body)
        }
    }
}

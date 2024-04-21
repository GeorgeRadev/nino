use crate::db_notification::{self, Notifier};
use crate::db_transactions::{QueryParam, TransactionSession};
use crate::nino_constants::info;
use crate::nino_structures::{JSTask, ServletTask};
use crate::web_responses::ResponseManager;
use crate::{nino_constants, nino_functions};
use async_channel::Receiver;
use deno_runtime::deno_core::{self, anyhow::Error, op2, Op, OpDecl, OpState};
use deno_runtime::deno_core::{JsBuffer, ToJsBuffer};
use deno_runtime::deno_fetch::reqwest::header::{HeaderName, HeaderValue};
use deno_runtime::deno_fetch::reqwest::{Body, Client, Method, Request};
use http_types::convert::Serialize;
use http_types::{StatusCode, Url};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::{cell::RefCell, rc::Rc};

pub fn get_nino_functions() -> Vec<OpDecl> {
    vec![
        nino_begin_task::DECL,
        nino_a_end_task::DECL,
        nino_a_sleep::DECL,
        nino_get_request::DECL,
        nino_get_request_body::DECL,
        nino_set_response_status::DECL,
        nino_set_response_header::DECL,
        nino_a_set_response_send_text::DECL,
        nino_a_set_response_send_buf::DECL,
        nino_get_invalidation_message::DECL,
        nino_get_thread_id::DECL,
        nino_broadcast_message::DECL,
        nino_a_broadcast_message::DECL,
        nino_get_module_invalidation_prefix::DECL,
        nino_get_database_invalidation_prefix::DECL,
        nino_reload_database_aliases::DECL,
        nino_tx_end::DECL,
        nino_tx_get_connection_name::DECL,
        nino_tx_execute_query::DECL,
        nino_tx_execute_upsert::DECL,
        nino_get_user_jwt::DECL,
        nino_password_hash::DECL,
        nino_password_verify::DECL,
        nino_a_fetch::DECL,
        nino_a_fetch_binary::DECL,
        nino_a_set_response_from_fetch::DECL,
    ]
}

pub struct JSContext {
    pub id: i16,
    pub dynamics: Arc<ResponseManager>,
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
// #[op2]
// async fn nino_async_task(state: Rc<RefCell<OpState>>) -> Result<(), Error> {
//     let future = tokio::task::spawn_blocking(move || {
//         // do some job here
//     });
//     future.await?;
//     Ok(())
// }

#[op2]
#[string]
fn nino_begin_task(state: &mut OpState) -> Result<String, Error> {
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

#[op2(fast)]
fn nino_tx_end(state: &mut OpState, commit: bool) {
    let tx = state.borrow_mut::<TransactionSession>();
    if let Err(error) = tx.close_all(commit) {
        eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
    }
}

#[op2(async)]
async fn nino_a_end_task(op_state: Rc<RefCell<OpState>>) -> Result<bool, Error> {
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequest {
    url: http_types::Url,
    original_url: String,
    method: String,
    host: String,
    path: String,
    query: String,
    parameters: HashMap<String, Vec<String>>,
    post_parameters: HashMap<String, Vec<String>>,
    user: String,
}

#[op2]
#[serde]
fn nino_get_request(state: &mut OpState) -> Result<HttpRequest, Error> {
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

                let mut post_parameters: HashMap<String, Vec<String>> = HashMap::new();
                if !servlet.body.contains(' ') {
                    let post_url_str = format!("{}?{}", nino_constants::MODULE_URI, servlet.body);
                    if let Ok(url) = Url::parse(&post_url_str) {
                        for (key, value) in url.query_pairs() {
                            let key = key.to_string();
                            let value = value.to_string();
                            match post_parameters.get_mut(&key) {
                                None => {
                                    let mut vec: Vec<String> = Vec::with_capacity(2);
                                    vec.push(value);
                                    post_parameters.insert(key, vec);
                                }
                                Some(vec) => {
                                    vec.push(value);
                                }
                            }
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
                    post_parameters,
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

#[op2(fast)]
fn nino_set_response_status(state: &mut OpState, status: u16) -> Result<(), Error> {
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

#[op2(fast)]
fn nino_set_response_header(
    state: &mut OpState,
    #[string] key: String,
    #[string] value: String,
) -> Result<(), Error> {
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

fn take_servlet_task(op_state: Rc<RefCell<OpState>>) -> Result<ServletTask, Error> {
    let mut state = op_state.borrow_mut();
    let context = state.borrow_mut::<JSContext>();

    if context.task.is_none() {
        //task already closed
        return Err(Error::msg("task already closed"));
    }

    match context.task.take().unwrap() {
        JSTask::Servlet(servlet) => Ok(servlet),
        JSTask::Message(_) => Err(Error::msg("task is not a request")),
    }
}

#[op2(async)]
async fn nino_a_set_response_send_text(
    op_state: Rc<RefCell<OpState>>,
    #[string] body: String,
) -> Result<(), Error> {
    let servlet_task = take_servlet_task(op_state)?;
    let mut response = servlet_task.response;
    let stream = servlet_task.stream;

    response.set_body(body);
    if let Err(error) = nino_functions::send_response_to_stream(stream, &mut response).await {
        eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
    };
    Ok(())
}

#[op2(async)]
async fn nino_a_set_response_send_buf(
    op_state: Rc<RefCell<OpState>>,
    #[buffer] bytes: JsBuffer,
) -> Result<(), Error> {
    let servlet_task = take_servlet_task(op_state)?;
    let mut response = servlet_task.response;
    let stream = servlet_task.stream;

    let data: &[u8] = &bytes;
    response.set_body(data);

    if let Err(error) = nino_functions::send_response_to_stream(stream, &mut response).await {
        eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
    };
    Ok(())
}

#[op2(async)]
async fn nino_a_sleep(op_state: Rc<RefCell<OpState>>, #[bigint] millis: u64) -> Result<(), Error> {
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

#[op2]
#[string]
fn nino_get_invalidation_message(state: &mut OpState) -> String {
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

#[op2(fast)]
fn nino_get_thread_id(state: &mut OpState) -> i16 {
    let context = state.borrow_mut::<JSContext>();
    context.id
}

#[op2(fast)]
fn nino_broadcast_message(state: &mut OpState, #[string] message: String) {
    let context = state.borrow_mut::<JSContext>();
    context.broadcast_messages.push(message);
}

#[op2(async)]
async fn nino_a_broadcast_message(op_state: Rc<RefCell<OpState>>, commit: bool) {
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
    if commit {
        for message in messages {
            if let Err(error) = notifier.notify(message).await {
                eprintln!("ERROR {}:{}:{}", function!(), line!(), error);
            }
        }
    }
}

#[op2]
#[string]
fn nino_get_module_invalidation_prefix() -> String {
    String::from(db_notification::NOTIFICATION_PREFIX_RESPONSE)
}

#[op2]
#[string]
fn nino_get_database_invalidation_prefix() -> String {
    String::from(db_notification::NOTIFICATION_PREFIX_DBNAME)
}

#[derive(Serialize)]
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
                Ok(v) => match chrono::DateTime::<chrono::Utc>::from_timestamp_millis(v) {
                    Some(dt) => {
                        let v = dt.to_rfc3339();
                        query_params.push(QueryParam::Date(v));
                    }
                    None => {
                        query_params.push(QueryParam::Null);
                    }
                },
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

#[op2(fast)]
fn nino_reload_database_aliases(state: &mut OpState) -> Result<(), Error> {
    let tx = state.borrow_mut::<TransactionSession>();
    tx.reload_database_aliases()
}

#[op2]
#[string]
fn nino_tx_get_connection_name(
    state: &mut OpState,
    #[string] db_alias: String,
) -> Result<String, Error> {
    let tx = state.borrow_mut::<TransactionSession>();
    tx.create_transaction(db_alias)
}

#[op2]
#[serde]
fn nino_tx_execute_query(
    state: &mut OpState,
    #[string] db_alias: String,
    #[serde] query: Vec<String>,
    #[serde] query_types: Vec<i16>,
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

#[op2]
#[bigint]
fn nino_tx_execute_upsert(
    state: &mut OpState,
    #[string] db_alias: String,
    #[serde] query: Vec<String>,
    #[serde] query_types: Vec<i16>,
) -> Result<u64, Error> {
    let (query, params) = query_types_to_params(query, query_types)?;
    let tx = state.borrow_mut::<TransactionSession>();
    tx.upsert(db_alias, query, params)
}

#[op2]
#[string]
fn nino_get_request_body(state: &mut OpState) -> Result<String, Error> {
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

#[op2]
#[string]
fn nino_get_user_jwt(_state: &mut OpState, #[string] user: String) -> Result<String, Error> {
    let mut map: HashMap<String, String> = HashMap::new();
    map.insert(nino_constants::JWT_USER.to_string(), user);
    nino_functions::jwt_from_map(nino_constants::PROGRAM_NAME, map)
}

#[op2]
#[string]
fn nino_password_hash(_state: &mut OpState, #[string] password: String) -> Result<String, Error> {
    nino_functions::password_hash(&password)
}

#[op2(fast)]
fn nino_password_verify(
    _state: &mut OpState,
    #[string] password: String,
    #[string] hash: String,
) -> Result<bool, Error> {
    nino_functions::password_verify(&password, &hash)
}

async fn fetch(
    url: String,
    timeout: i64,
    method: String,
    headers: HashMap<String, String>,
    body: String,
) -> Result<deno_runtime::deno_fetch::reqwest::Response, Error> {
    // Build out the request
    let url = Url::from_str(&url)?;
    let method = Method::from_bytes(method.as_bytes())?;
    let mut request = Request::new(method, url);

    for (key, value) in headers.iter() {
        request.headers_mut().insert(
            HeaderName::from_str(key).unwrap(),
            HeaderValue::from_str(value).unwrap(),
        );
    }
    request.body_mut().replace(Body::from(body));

    let response_future = {
        // let https = HttpsConnector::new();
        let client = Client::new();
        client.execute(request)
    };

    match tokio::time::timeout(
        tokio::time::Duration::from_millis(timeout as u64),
        response_future,
    )
    .await
    {
        Err(_) => Err(Error::msg("Connection Timeout")),
        Ok(Err(e)) => Err(e.into()),
        Ok(Ok(response)) => Ok(response),
    }
}

#[op2(async)]
#[string]
async fn nino_a_fetch(
    _op_state: Rc<RefCell<OpState>>,
    #[string] url: String,
    #[bigint] timeout: i64,
    #[string] method: String,
    #[serde] headers: HashMap<String, String>,
    #[string] body: String,
) -> Result<String, Error> {
    let response = fetch(url, timeout, method, headers, body).await?;
    let bytes = response.bytes().await?.to_vec();
    let body = String::from_utf8(bytes)?;
    Ok(body)
}

#[op2(async)]
#[serde]
async fn nino_a_fetch_binary(
    _op_state: Rc<RefCell<OpState>>,
    #[string] url: String,
    #[bigint] timeout: i64,
    #[string] method: String,
    #[serde] headers: HashMap<String, String>,
    #[string] body: String,
) -> Result<ToJsBuffer, Error> {
    let response = fetch(url, timeout, method, headers, body).await?;
    let bytes = response.bytes().await?.to_vec();
    Ok(bytes.into())
}

#[op2(async)]
async fn nino_a_set_response_from_fetch(
    op_state: Rc<RefCell<OpState>>,
    #[string] url: String,
    #[bigint] timeout: i64,
    #[string] method: String,
    #[serde] headers: HashMap<String, String>,
    #[string] body: String,
) -> Result<(), Error> {
    let response_in = fetch(url, timeout, method, headers, body).await?;

    let servlet_task = take_servlet_task(op_state)?;
    let stream_out = servlet_task.stream;

    nino_functions::send_request_to_stream(response_in, stream_out).await
}

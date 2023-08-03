use crate::db_notification::{self, Notifier};
use crate::js_dbs::JSDBManager;
use crate::nino_structures;
use crate::web_dynamics::DynamicManager;
use crate::{db::DBManager, nino_functions};
use async_channel::Receiver;
use async_std::net::TcpStream;
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
        aop_jsdb_get_connection_name::DECL,
        aop_jsdb_execute_query::DECL,
        aop_jsdb_execute_query_one::DECL,
    ]
}

pub struct JSContext {
    pub id: u32,
    pub db: Arc<DBManager>,
    pub jsdb: Arc<JSDBManager>,
    pub dynamics: Arc<DynamicManager>,
    pub notifier: Arc<Notifier>,
    pub web_task_rx: Receiver<Box<nino_structures::WebTask>>,
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
    pub fn close(&mut self) {
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
        self.jsdb.cleanup(false);
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

            println!("new js task");
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
async fn aop_end_task(op_state: Rc<RefCell<OpState>>, _error: bool) -> Result<bool, Error> {
    let stream;
    let mut response;
    {
        let mut state = op_state.borrow_mut();
        let context = state.borrow_mut::<JSContext>();
        /*
        if error {
            // rollback
        } else {
            //commit dbs
        }
        */

        if context.closed {
            //task already closed
            return Ok(false);
        }

        stream = context.stream.take().unwrap();
        response = context.response.take().unwrap();

        context.close();
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
    aop_set_response_send(op_state, "plain/text;charset=UTF-8", body).await
}

#[op]
async fn aop_set_response_send_json(
    op_state: Rc<RefCell<OpState>>,
    body: String,
) -> Result<(), Error> {
    aop_set_response_send(op_state, "application/json", body).await
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
        context.close();
    }

    let has_no_type = response.header(CONTENT_TYPE).is_none();
    response.set_body(body);
    if has_no_type {
        response.insert_header(CONTENT_TYPE, mime);
    }

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
        context.close();
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
fn op_get_thread_id(state: &mut OpState) -> u32 {
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
async fn aop_jsdb_get_connection_name(
    op_state: Rc<RefCell<OpState>>,
    db_alias: String,
) -> Result<String, Error> {
    let jsdb;
    {
        let mut state = op_state.borrow_mut();
        let context = state.borrow_mut::<JSContext>();
        jsdb = context.jsdb.clone();
    }
    jsdb.create_db_connection(db_alias).await
}

#[op]
async fn aop_jsdb_execute_query(
    op_state: Rc<RefCell<OpState>>,
    db_alias: String,
    query: Vec<String>,
    query_types: Vec<i16>,
) -> Result<Vec<Vec<String>>, Error> {
    let jsdb;
    {
        let mut state = op_state.borrow_mut();
        let context = state.borrow_mut::<JSContext>();
        jsdb = context.jsdb.clone();
    }
    jsdb.execute_query(db_alias, query, query_types).await
}

#[op]
async fn aop_jsdb_execute_query_one(
    op_state: Rc<RefCell<OpState>>,
    db_alias: String,
    query: Vec<String>,
    query_types: Vec<i16>,
) -> Result<Vec<String>, Error> {
    let jsdb;
    {
        let mut state = op_state.borrow_mut();
        let context = state.borrow_mut::<JSContext>();
        jsdb = context.jsdb.clone();
    }
    jsdb.execute_query_one(db_alias, query, query_types).await
}

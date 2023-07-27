use crate::nino_structures;
use crate::web_dynamics::DynamicManager;
use crate::{db::DBManager, nino_functions};
use async_channel::Receiver;
use deno_core::{Op, op, anyhow::Error, OpDecl, OpState};
use http_types::headers::CONTENT_TYPE;
use http_types::{Response, StatusCode};
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
    ]
}

pub struct JSTask {
    pub id: u32,
    pub db: DBManager,
    pub web_task_rx: Receiver<Box<nino_structures::WebTask>>,
    pub web_task: Option<Box<nino_structures::WebTask>>,
    // response
    pub is_request: bool,
    pub response: Response,
    pub dynamics: Arc<DynamicManager>,
    pub module: String,
    pub closed: bool,
    // invalidate
    pub is_invalidate: bool,
    pub message: Option<String>,
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

#[op]
fn op_begin_task(op_state: &mut OpState) -> Result<String, Error> {
    let inner_state = op_state.borrow_mut::<JSTask>();
    //return Ok(inner_state.module.clone());
    let result = inner_state.web_task_rx.recv_blocking();
    let mut module = String::from("");
    match result {
        Ok(web_task) => {
            // request
            inner_state.is_request = web_task.is_request;
            if web_task.js_module.is_some() {
                module = web_task.js_module.clone().unwrap();
            }
            inner_state.response = Response::new(200);
            // invalidate
            inner_state.is_request = web_task.is_invalidate;
            if web_task.is_invalidate {
                inner_state.message = web_task.message.clone();
                inner_state.closed = true;
            } else {
                inner_state.message = None;
                inner_state.closed = false;
            }
            inner_state.web_task = Some(web_task);

            println!("new js task");
        }
        Err(error) => {
            inner_state.closed = true;
            println!(
                "{}:{}:{} new js task ERROR: {}",
                function!(),
                line!(),
                inner_state.id,
                error.to_string()
            );
        }
    }
    Ok(module)
}

#[op]
async fn aop_end_task(state: Rc<RefCell<OpState>>) -> Result<bool, Error> {
    let mut op_state = state.borrow_mut();
    let inner_state = op_state.borrow_mut::<JSTask>();
    if inner_state.closed {
        //task already closed
        return Ok(false);
    }

    let web_task = inner_state.web_task.as_mut().unwrap();
    if let Err(error) = nino_functions::send_response_to_stream(
        &mut web_task.stream.as_mut().unwrap(),
        &mut inner_state.response,
    )
    .await
    {
        eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
    }
    inner_state.closed = true;
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
    let task = state.borrow_mut::<JSTask>();
    let web_task = task.web_task.as_mut().unwrap();
    if !web_task.is_request {
        return Err(Error::msg("task is not a request"));
    }
    let request = web_task.request.as_mut().unwrap();
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
    let task = state.borrow_mut::<JSTask>();
    task.response
        .set_status(StatusCode::try_from(status).unwrap());
    Ok(())
}

#[op]
fn op_set_response_header(state: &mut OpState, key: String, value: String) -> Result<(), Error> {
    let task = state.borrow_mut::<JSTask>();
    task.response.remove_header(&*key);
    task.response.append_header(&*key, &*value);
    Ok(())
}

#[op]
async fn aop_set_response_send_text(
    state: Rc<RefCell<OpState>>,
    body: String,
) -> Result<(), Error> {
    aop_set_response_send(state, "plain/text;charset=UTF-8", body).await
}

#[op]
async fn aop_set_response_send_json(
    state: Rc<RefCell<OpState>>,
    body: String,
) -> Result<(), Error> {
    aop_set_response_send(state, "application/json", body).await
}

async fn aop_set_response_send(
    state: Rc<RefCell<OpState>>,
    mime: &str,
    body: String,
) -> Result<(), Error> {
    let mut op_state = state.borrow_mut();
    let inner_state = op_state.borrow_mut::<JSTask>();
    let response = &mut inner_state.response;

    let has_no_type = response.header(CONTENT_TYPE).is_none();
    response.set_body(body);
    if has_no_type {
        response.insert_header(CONTENT_TYPE, mime);
    }

    let web_task = inner_state.web_task.as_mut().unwrap().as_mut();
    if !web_task.is_request {
        return Err(Error::msg("task is not a request"));
    }
    if let Err(error) = nino_functions::send_response_to_stream(
        web_task.stream.as_mut().unwrap(),
        &mut inner_state.response,
    )
    .await
    {
        eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
    };
    inner_state.closed = true;
    Ok(())
}

#[op]
async fn aop_set_response_send_buf(
    state: Rc<RefCell<OpState>>,
    buffer: Vec<u8>,
) -> Result<(), Error> {
    let mut op_state = state.borrow_mut();
    let inner_state = op_state.borrow_mut::<JSTask>();
    let response = &mut inner_state.response;

    let has_no_type = response.header(CONTENT_TYPE).is_none();
    response.set_body(buffer);
    if has_no_type {
        response.insert_header(CONTENT_TYPE, "text/html;charset=UTF-8");
    }

    let web_task = inner_state.web_task.as_mut().unwrap().as_mut();
    if !web_task.is_request {
        return Err(Error::msg("task is not a request"));
    }
    if let Err(error) = nino_functions::send_response_to_stream(
        web_task.stream.as_mut().unwrap(),
        &mut inner_state.response,
    )
    .await
    {
        eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
    };
    inner_state.closed = true;
    Ok(())
}

#[op]
async fn aop_sleep(state: Rc<RefCell<OpState>>, millis: u64) -> Result<(), Error> {
    // Future must be Poll::Pending on first call
    let v;
    {
        let op_state = state.borrow();
        let task = op_state.borrow::<JSTask>();
        v = task.id;
    }
    println!("{} waiting", v);
    tokio::time::sleep(std::time::Duration::from_millis(millis)).await;
    Ok(())
}

#[op]
fn op_get_invalidation_message(state: &mut OpState) -> String {
    let task = state.borrow_mut::<JSTask>();
    if task.is_invalidate {
        if let Some(message) = task.message.clone() {
            return message;
        }
    }
    return String::from("");
}

#[op]
fn op_get_thread_id(state: &mut OpState) -> u32 {
    let task = state.borrow_mut::<JSTask>();
    task.id
}

// #[op]
// async fn op_async_task(state: Rc<RefCell<OpState>>) -> Result<(), Error> {
//     let future = tokio::task::spawn_blocking(move || {
//         // do some job here
//     });
//     future.await?;
//     Ok(())
// }

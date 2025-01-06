use crate::nino_constants::info;
use async_std::net::TcpStream;
use http_types::{Request, Response};

#[derive(Clone)]
pub struct InitialSettings {
    pub system_id: String,
    pub connection_string: String,
    pub thread_count: usize,
    pub db_pool_size: usize,
    pub js_thread_count: usize,
    pub debug_port: u16,
}

impl InitialSettings {
    pub fn print(&self) {
        info!("system_id: {}", self.system_id);
        info!("thread_count: {}", self.thread_count);
        info!("db_pool_size: {}", self.db_pool_size);
        info!("js_thread_count: {}", self.js_thread_count);
        info!("debug_port: {}", self.debug_port);
        // skiping db connection log for security reasons
    }
}

#[derive(Clone)]

pub enum JSTask {
    Servlet(ServletTask),
    Message(String),
}

#[derive(Clone)]
pub struct ServletTask {
    pub method: String,
    pub request_path: String,
    pub js_module: Option<String>,
    pub request: Request,
    pub user: String,
    pub body: String,
    pub response: Option<Response>,
    pub stream: Box<TcpStream>,
}

#[derive(Clone)]
pub struct NotificationMessage {
    pub text: String,
}

#[derive(Clone)]
pub struct LogInfo {
    pub method: String,
    pub request: String,
    pub response: String,
    pub message: String,
}

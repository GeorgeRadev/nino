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

#[derive(Clone)]

pub enum JSTask {
    Servlet(ServletTask),
    Message(String),
}

#[derive(Clone)]
pub struct ServletTask {
    pub js_module: String,
    pub request: Request,
    pub user: String,
    pub body: String,
    pub response: Response,
    pub stream: Box<TcpStream>,
}

#[derive(Clone)]
pub struct NotificationMessage {
    pub text: String,
}

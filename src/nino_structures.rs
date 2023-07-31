use async_std::net::TcpStream;
use http_types::Request;

#[derive(Clone)]
pub struct InitialSettings {
    pub connection_string: String,
    pub thread_count: usize,
    pub db_pool_size: usize,
    pub js_thread_count: usize,
    pub server_port: u16,
    pub debug_port: u16,
}

#[derive(Clone)]
pub struct WebTask {
    //request task
    pub is_request: bool,
    pub js_module: Option<String>,
    pub request: Option<Request>,
    pub stream: Option<Box<TcpStream>>,
    // invalidate task
    pub is_invalidate: bool,
    pub message: String,
}

#[derive(Clone)]
pub struct Message {
    pub text: String,
}

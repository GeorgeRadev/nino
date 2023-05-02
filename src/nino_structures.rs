use async_std::net::TcpStream;
use http_types::Request;

#[derive(Clone)]
pub struct InitialSettings {
    pub connection_string: String,
    pub thread_count: usize,
    pub db_pool_size: usize,
    pub js_thread_count: u16,
    pub server_port: u16,
}

#[derive(Clone)]
pub struct WebTask {
    pub js_module: String,
    pub request: Request,
    pub stream: Box<TcpStream>,
}

#[derive(Clone)]
pub struct Message {
    pub json: String,
}

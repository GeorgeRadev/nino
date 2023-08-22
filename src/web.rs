use crate::nino_constants::info;
use crate::nino_functions;
use crate::web_dynamics::DynamicManager;
use crate::web_requests::RequestManager;
use crate::web_statics::StaticManager;
use async_std::net::{TcpListener, TcpStream};
use deno_core::anyhow::Error;
use http_types::{Request, Response, StatusCode, Url};
use std::net::SocketAddr;
use std::sync::Arc;

/// A Web Server with dispatching requests to static and dynamic manager
pub struct WebManager {
    port: u16,
    requests: Arc<RequestManager>,
    statics: Arc<StaticManager>,
    dynamics: Arc<DynamicManager>,
}

impl WebManager {
    pub fn new(
        port: u16,
        requests: Arc<RequestManager>,
        statics: Arc<StaticManager>,
        dynamics: Arc<DynamicManager>,
    ) -> WebManager {
        WebManager {
            port,
            requests,
            statics,
            dynamics,
        }
    }

    pub async fn start(&self) -> Result<(), Error> {
        let listener = TcpListener::bind(("127.0.0.1", self.port)).await?;
        println!("starting HTTP server at http://localhost:{}", self.port);
        let bl = Box::new(listener);
        Self::listening(
            bl,
            self.requests.clone(),
            self.statics.clone(),
            self.dynamics.clone(),
        )
        .await;
        Ok(())
    }

    async fn listening(
        listener: Box<TcpListener>,
        requests: Arc<RequestManager>,
        statics: Arc<StaticManager>,
        dynamics: Arc<DynamicManager>,
    ) -> ! {
        // serving loop
        loop {
            let conn = listener.accept().await;
            match conn {
                Ok((stream, _socket_addr)) => {
                    // spawn new task
                    tokio::task::spawn(Self::serve_request(
                        Box::new(stream),
                        requests.clone(),
                        statics.clone(),
                        dynamics.clone(),
                    ));
                }
                Err(error) => {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                }
            }
        }
    }

    async fn serve_request(
        stream: Box<TcpStream>,
        requests: Arc<RequestManager>,
        statics: Arc<StaticManager>,
        dynamics: Arc<DynamicManager>,
    ) {
        let from_addres = match stream.peer_addr() {
            Ok(address) => address,
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                return;
            }
        };
        // info!("starting new connection from {}", from_addres);
        /* match timeout(timeout_duration, fut).await {
            Ok(Ok(Some(r))) => r,
            Ok(Ok(None)) | Err(TimeoutError { .. }) => return Ok(ConnectionStatus::Close), /* EOF or timeout */
            Ok(Err(e)) => return Err(e),
        }*/
        match async_h1::server::decode(stream.clone()).await {
            Ok(result) => {
                if let Some(request) = result {
                    let (request, _) = request;
                    // queue task with request
                    if let Err(error) = Self::dispatch_request(
                        from_addres,
                        request,
                        stream.clone(),
                        requests,
                        statics,
                        dynamics,
                    )
                    .await
                    {
                        // requestor has closed the stream
                        info!("ERROR {}:{}:{}", file!(), line!(), error);
                        //Self::response_500(stream, error).await;
                    }
                    return;
                }
            }
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            }
        }
        //invalid/unrecognized request
        //close connection
        let _r = stream.shutdown(std::net::Shutdown::Both);
    }

    async fn dispatch_request(
        from_address: SocketAddr,
        mut request: Request,
        stream: Box<TcpStream>,
        requests: Arc<RequestManager>,
        statics: Arc<StaticManager>,
        dynamics: Arc<DynamicManager>,
    ) -> Result<(), Error> {
        let method = request.method();
        let url = request.url().clone();
        let path = nino_functions::normalize_path(url.path().to_string());
        
        println!("REQUEST: {} {} {}", method, from_address, url);
        
        match requests.get_request(&path).await? {
            None => {
                Self::response_404(stream, &url).await;
                Ok(())
            }
            Some(request_info) => {
                if request_info.redirect {
                    Self::response_307_redirect(stream, &request_info.name).await;
                    Ok(())
                } else if !request_info.dynamic {
                    //serve static resources
                    statics
                    .serve_static(request_info, request.clone(), stream.clone())
                    .await
                    //ok - stream should be served and closed
                } else {
                    // serve from dynamic resources
                    if request_info.execute {
                        // execute the JS
                        let body = request.body_string().await.map_err(|e| Error::msg(e.to_string()))?;
                        dynamics
                        .execute_dynamic(request_info, request.clone(), stream.clone(), body)
                        .await
                        //ok - stream should be served and closed
                    } else {
                        // return js code as response
                        dynamics.serve_dynamic(request_info, stream.clone()).await
                        //ok - stream should be served and closed
                    }
                }
            }
        }
    }

    async fn response_307_redirect(stream: Box<TcpStream>, url: &String) {
        // TODO: introduce 404 handler
        let mut response = Response::new(StatusCode::TemporaryRedirect);
        response.append_header("Location", url);
        if let Err(error) = nino_functions::send_response_to_stream(stream, &mut response).await {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
        }
    }
    async fn response_404(stream: Box<TcpStream>, url: &Url) {
        // TODO: introduce 404 handler
        let mut response = Response::new(StatusCode::NotFound);
        let content = format!("url not found: {} ", url);
        response.set_body(http_types::Body::from_string(content));
        if let Err(error) = nino_functions::send_response_to_stream(stream, &mut response).await {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
        }
    }
    /*
    async fn response_500(stream: Box<TcpStream>, error: Error) {
        let mut response = Response::new(StatusCode::InternalServerError);
        let content = format!("ERROR: {}", error);
        response.set_body(http_types::Body::from_string(content));
        if let Err(error) = nino_functions::send_response_to_stream(stream, &mut response).await {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
        }
    }
    */
}

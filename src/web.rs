use crate::nino_functions;
use crate::web_dynamics::DynamicsManager;
use crate::web_statics::StaticsManager;
use async_std::net::{TcpListener, TcpStream};
use http_types::{Request, Response, StatusCode};
use std::net::SocketAddr;
use std::sync::Arc;

/// A Web Server with dispatching requests to static and dynamic manager
pub struct WebManager {
    port: u16,
    statics: Arc<StaticsManager>,
    dynamics: Arc<DynamicsManager>,
}

impl WebManager {
    pub fn new(
        port: u16,
        statics: Arc<StaticsManager>,
        dynamics: Arc<DynamicsManager>,
    ) -> WebManager {
        WebManager {
            port,
            statics,
            dynamics,
        }
    }

    pub async fn start(&self) -> Result<(), String> {
        let listener = TcpListener::bind(("127.0.0.1", self.port))
            .await
            .map_err(|e| e.to_string())?;
        println!("starting HTTP server at http://localhost:{}", self.port);
        let bl = Box::new(listener);
        Self::listening(bl, self.statics.clone(), self.dynamics.clone()).await;
        Ok(())
    }

    async fn listening(
        listener: Box<TcpListener>,
        statics: Arc<StaticsManager>,
        dynamics: Arc<DynamicsManager>,
    ) -> ! {
        // serving loop
        loop {
            let conn = listener.accept().await;
            match conn {
                Ok((stream, _socket_addr)) => {
                    // spawn new task
                    tokio::task::spawn(Self::serve_request(
                        Box::new(stream),
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
        statics: Arc<StaticsManager>,
        dynamics: Arc<DynamicsManager>,
    ) {
        let from_addres = match stream.peer_addr() {
            Ok(address) => address,
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error.to_string());
                return;
            }
        };
        // println!("starting new connection from {}", from_addres);
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
                    Self::dispatch_request(from_addres, request, stream.clone(), statics, dynamics)
                        .await;
                    return;
                }
            }
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error.to_string());
            }
        }
        //invalid/unrecognized request
        //close connection
        let _r = stream.shutdown(std::net::Shutdown::Both);
        return;
    }

    async fn dispatch_request(
        from_address: SocketAddr,
        request: Request,
        stream: Box<TcpStream>,
        statics: Arc<StaticsManager>,
        dynamics: Arc<DynamicsManager>,
    ) {
        let method = request.method();
        let url = request.url();
        let path_str = nino_functions::normalize_path(url.path().to_string());
        let path = path_str.as_str();

        println!("REQUEST: {} {} {}", method, from_address, url);

        if statics
            .serve_static(path, request.clone(), stream.clone())
            .await
        {
            //ok - sream should be served and closed
        } else if dynamics
            .serve_dynamic(path, request.clone(), stream.clone())
            .await
        {
            //ok - sream should be served and closed
        } else {
            // return 404 not found
            // TODO: introduce 404 handler
            let mut response = Response::new(StatusCode::NotFound);
            let content = format!("url not found: {} ", url);
            response.set_body(http_types::Body::from_string(content));
            if let Err(error) =
                nino_functions::send_response_to_stream(stream.clone().as_mut(), &mut response)
                    .await
            {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            }
        }
    }
}

use std::net::SocketAddr;

use anyhow::Error;
use async_std::net::{TcpListener, TcpStream};
use http_types::{Request, Response, StatusCode, Url};

fn main() {
    println!("Hello, world!");

    // async functionalities goes here
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(main_async());
}

async fn main_async() {
    if let Err(error) = start(8888).await {
        eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
        std::process::exit(1);
    }
}

pub async fn start(port: u16) -> Result<(), Error> {
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    println!("starting HTTP server at http://localhost:{}", port);
    listening(listener, 2000).await;
}

async fn listening(listener: TcpListener, request_timeout_ms: u32) -> ! {
    // serving loop
    loop {
        let conn = listener.accept().await;
        match conn {
            Ok((stream, _socket_addr)) => {
                // spawn new task
                tokio::task::spawn(serve_request(request_timeout_ms, stream));
            }
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            }
        }
    }
}

async fn serve_request(request_timeout_ms: u32, stream: TcpStream) {
    // let (read_stream, mut write_stream) = stream.into_split();
    let from_addres = match stream.peer_addr() {
        Ok(address) => address,
        Err(error) => {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            return;
        }
    };
    // add request timeout - to avoid slow lorry attacks
    match tokio::time::timeout(
        tokio::time::Duration::from_millis(request_timeout_ms as u64),
        async_h1::server::decode(stream.clone()),
    )
    .await
    {
        Err(error) => println!("ERROR {}:{}:{}", file!(), line!(), error),
        Ok(Err(error)) => println!("ERROR {}:{}:{}", file!(), line!(), error),
        Ok(Ok(request)) => {
            match request {
                Some(request) => {
                    let (request, _) = request;
                    // queue task with request
                    if let Err(error) = dispatch_request(from_addres, request, stream).await {
                        // requestor has closed the stream
                        println!("ERROR {}:{}:{}", file!(), line!(), error);
                    }
                    return;
                }
                None => {
                    // requestor has closed the stream
                    // info!("ERROR {}:{}:{}", file!(), line!(), "should not happen");
                }
            }
        }
    }
    //invalid/unrecognized request
    //close connection
    let _r = stream.shutdown(std::net::Shutdown::Both);
}

async fn dispatch_request(
    from_address: SocketAddr,
    request: Request,
    stream: TcpStream,
) -> Result<(), Error> {
    let method = request.method();
    let url = request.url().clone();

    println!("REQUEST: {} {} {}", method, from_address, url);

    response_404(stream, &url).await;
    Ok(())
}

async fn response_404(stream: TcpStream, url: &Url) {
    // TODO: introduce 404 handler
    let mut response = Response::new(StatusCode::NotFound);
    let content = format!("url not found: {} ", url);
    response.set_body(http_types::Body::from_string(content));
    if let Err(error) = send_response_to_stream(stream, response).await {
        eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
    }
}

const HTTP: &str = "HTTP/1.1";
const CRLF: &str = "\r\n";
const SEPARATOR: &str = ": ";
const CONTENT_LENGTH: &str = "Content-Length";

async fn send_response_to_stream(
    stream: TcpStream,
    mut response: Response,
) -> Result<(), Error> {
    match response.body_bytes().await {
        Err(error) => {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
        }
        Ok(body) => {
            if response.header(CONTENT_LENGTH).is_none() {
                response.insert_header(CONTENT_LENGTH, format!("{}", body.len()));
            }

            //write status
            let mut header_string = String::with_capacity(1024);
            header_string.push_str(HTTP);
            header_string.push(' ');
            header_string.push_str(&format!("{}", response.status()));
            header_string.push(' ');
            header_string.push_str(response.status().canonical_reason());
            header_string.push_str(CRLF);

            // write header
            for (header_key, header_value) in response.iter() {
                header_string.push_str(header_key.as_str());
                header_string.push_str(SEPARATOR);
                header_string.push_str(header_value.as_str());
                header_string.push_str(CRLF);
            }

            //write separtor
            header_string.push_str(CRLF);

            //write body
            {
                let mut http_bytes = header_string.as_bytes();
                async_std::io::copy(&mut http_bytes, &mut stream.clone()).await?;
                async_std::io::copy(&mut body.as_slice(), &mut stream.clone()).await?;
            }
        }
    };

    //close socket - always
    if stream.shutdown(std::net::Shutdown::Both).is_err() {
        // stream already closed
    }
    Ok(())
}


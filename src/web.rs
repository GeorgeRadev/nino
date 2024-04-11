use crate::db_settings::SettingsManager;
use crate::nino_constants::{
    self, info, SETTINGS_NINO_LOGIN_PATH, SETTINGS_NINO_LOGIN_PATH_DEFAULT,
};
use crate::nino_functions;
use crate::web_requests::RequestManager;
use crate::web_responses::ResponseManager;
use async_std::net::{TcpListener, TcpStream};
use deno_runtime::deno_core::anyhow::{self, Error};
use http_types::headers::HeaderValues;
use http_types::{Method, Request, Response, StatusCode, Url};
use std::net::SocketAddr;
use std::sync::Arc;

/// A Web Server with dispatching requests to static and dynamic manager
pub struct WebManager {
    port: u16,
    request_timeout_ms: u32,
    settings: Arc<SettingsManager>,
    requests: Arc<RequestManager>,
    responses: Arc<ResponseManager>,
}

impl WebManager {
    pub async fn new(
        settings: Arc<SettingsManager>,
        requests: Arc<RequestManager>,
        responses: Arc<ResponseManager>,
    ) -> WebManager {
        let port = settings
            .get_setting_i32(
                nino_constants::SETTINGS_NINO_WEB_SERVER_PORT,
                nino_constants::SETTINGS_NINO_WEB_SERVER_PORT_DEFAULT,
            )
            .await as u16;
        let request_timeout_ms = settings
            .get_setting_i32(
                nino_constants::SETTINGS_NINO_WEB_REQUEST_TIMEOUT,
                nino_constants::SETTINGS_NINO_WEB_REQUEST_TIMEOUT_DEFAULT,
            )
            .await as u32;
        WebManager {
            port,
            request_timeout_ms,
            settings,
            requests,
            responses,
        }
    }

    pub async fn start(&self) -> Result<(), Error> {
        let listener = TcpListener::bind(("127.0.0.1", self.port)).await?;
        println!("starting HTTP server at http://localhost:{}", self.port);
        let bl = Box::new(listener);
        Self::listening(
            bl,
            self.request_timeout_ms,
            self.settings.clone(),
            self.requests.clone(),
            self.responses.clone(),
        )
        .await;
        Ok(())
    }

    async fn listening(
        listener: Box<TcpListener>,
        request_timeout_ms: u32,
        settings: Arc<SettingsManager>,
        requests: Arc<RequestManager>,
        responses: Arc<ResponseManager>,
    ) -> ! {
        // serving loop
        loop {
            let conn = listener.accept().await;
            match conn {
                Ok((stream, _socket_addr)) => {
                    // spawn new task
                    tokio::task::spawn(Self::serve_request(
                        request_timeout_ms,
                        Box::new(stream),
                        settings.clone(),
                        requests.clone(),
                        responses.clone(),
                    ));
                }
                Err(error) => {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                }
            }
        }
    }

    async fn serve_request(
        request_timeout_ms: u32,
        stream: Box<TcpStream>,
        settings: Arc<SettingsManager>,
        requests: Arc<RequestManager>,
        responses: Arc<ResponseManager>,
    ) {
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
            Err(error) => info!("OK {}:{}:{}", file!(), line!(), error),
            Ok(Err(error)) => info!("ERROR {}:{}:{}", file!(), line!(), error),
            Ok(Ok(request)) => {
                match request {
                    Some(request) => {
                        let (request, _) = request;
                        // queue task with request
                        if let Err(error) = Self::dispatch_request(
                            from_addres,
                            request,
                            stream.clone(),
                            settings,
                            requests,
                            responses,
                        )
                        .await
                        {
                            // requestor has closed the stream
                            info!("ERROR {}:{}:{}", file!(), line!(), error);
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
        mut request: Request,
        stream: Box<TcpStream>,
        settings: Arc<SettingsManager>,
        requests: Arc<RequestManager>,
        responses: Arc<ResponseManager>,
    ) -> Result<(), Error> {
        let method = request.method();
        let url = request.url().clone();
        let path = nino_functions::normalize_path(url.path().to_string());
        let mut current_user = String::new();

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
                } else if request_info.authorize
                    && !Self::check_authorization(&request, &mut current_user)
                {
                    // redirect to login
                    // TODO: add this as parameter
                    let mut redirect_url = url.clone();
                    let login_path = settings
                        .get_setting_str(SETTINGS_NINO_LOGIN_PATH, SETTINGS_NINO_LOGIN_PATH_DEFAULT)
                        .await;
                    redirect_url.set_path(&login_path);
                    Self::response_307_redirect(stream, &redirect_url.into()).await;
                    Ok(())
                } else {
                    let response_info = responses.get_response(&request_info.name).await?;
                    match response_info {
                        None => {
                            // no response defined
                            eprintln!(
                                "ERROR {}:{}:{}",
                                file!(),
                                line!(),
                                anyhow::anyhow!(
                                    "No response matching the request path: {}",
                                    request_info.name
                                )
                            );
                            Self::response_404(stream, &url).await;
                            Ok(())
                        }
                        Some(response_info) => {
                            if response_info.execute {
                                // execute the JS
                                let body = request
                                    .body_string()
                                    .await
                                    .map_err(|e| Error::msg(e.to_string()))?;
                                responses
                                    .serve_dynamic(
                                        request,
                                        &request_info,
                                        &response_info,
                                        stream.clone(),
                                        current_user,
                                        body,
                                    )
                                    .await
                                //ok - stream should be served and closed
                            } else {
                                // return static as response
                                let method = request.method();
                                if Method::Get != method {
                                    // handle only GET static requests
                                    return Err(Error::msg(
                                        "static requests handles only GET requests",
                                    ));
                                }
                                responses
                                    .serve_static(request_info, response_info, stream.clone())
                                    .await
                                //ok - stream should be served and closed
                            }
                        }
                    }
                }
            }
        }
    }

    fn check_cookie(cookies: &HeaderValues, current_user: &mut String) -> bool {
        // TODO: add config for this
        let cookie_prefix = "nino=";
        for cookie in cookies.iter() {
            let cookie_tokens = cookie.as_str().split(";");
            for cookie_token in cookie_tokens {
                let token = cookie_token.trim();
                if token.starts_with(cookie_prefix) {
                    let jwt = &token[cookie_prefix.len()..];
                    if Self::jwt_to_user(jwt, current_user) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn check_authorization(request: &Request, current_user: &mut String) -> bool {
        //check header for session cookie or athorization header value
        //header : first "Cookie: nino=" then "Authorization: Bearer "
        if let Some(cookies) = request.header("cookie") {
            if Self::check_cookie(cookies, current_user) {
                return true;
            }
        }
        if let Some(cookies) = request.header("Cookie") {
            if Self::check_cookie(cookies, current_user) {
                return true;
            }
        }
        if let Some(authorizations) = request.header("Authorization") {
            // TODO: add config for this
            let auth_prefix = "Bearer ";
            for authorization in authorizations.iter() {
                if authorization.as_str().starts_with(auth_prefix) {
                    let jwt = authorization.as_str()[auth_prefix.len()..].trim();
                    if Self::jwt_to_user(jwt, current_user) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn jwt_to_user(jwt: &str, current_user: &mut String) -> bool {
        // check if jwt is valid
        // TODO: add config for this
        match nino_functions::jwt_to_map(nino_constants::PROGRAM_NAME, jwt) {
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                false
            }
            Ok(map) => {
                // jwt is valid
                if let Some(username) = map.get(nino_constants::JWT_USER) {
                    current_user.clear();
                    current_user.push_str(username);
                }
                true
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

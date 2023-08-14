use async_std::net::TcpStream;
use deno_core::anyhow::Error;
use http_types::Response;

use crate::nino_constants;

/// Get the postgres connection string from the
/// program parameters or system environment variable (in dat order of existance).
/// the name of the program is used as name on the environment parameter (ex nino)
pub fn get_connection_string() -> Result<String, String> {
    use std::env;

    let mut args_iter = env::args();
    // program name is usiality the zero parameter
    match args_iter.next() {
        Some(_) => {
            // read the next parameter
            match args_iter.next() {
                // the first program parameter should be the connection string
                Some(connection_string) => Ok(connection_string),
                None => {
                    // try getting from NINO environment variable
                    let name_upper = nino_constants::PROGRAM_NAME.to_string().to_uppercase();
                    match env::var(name_upper) {
                        Ok(connection_string) => Ok(connection_string),
                        Err(e) => Err(e.to_string()),
                    }
                }
            }
        }
        None => Err("No program name".to_string()),
    }
}

/// remove any strange paths and characters
/// use only lowercase, slash, dot and underscore
/// where dot and slash can only be a single one for path reference avoidance
pub fn normalize_path(path: String) -> String {
    let mut result: Vec<char> = Vec::with_capacity(path.len());
    let mut prev: char = '\0';
    for b in path.to_lowercase().chars() {
        let is_special_char = b == '_' || b == '/' || b == '.';
        if (is_special_char && (b == prev)) || (prev == '/' && b == '.') {
            //do not duplicate special chars, and
            //do not allow adding relative paths
        } else {
            result.push(b);
        }
        prev = b;
    }
    while !result.is_empty() && (*result.last().unwrap() == '/') {
        result.pop();
    }
    while !result.is_empty() && (*result.first().unwrap() == '/') {
        result.remove(0);
    }
    if result.is_empty() {
        result.push('/');
    }
    result.into_iter().collect()
}

const HTTP: &str = "HTTP/1.1";
const CRLF: &str = "\r\n";
const SEPARATOR: &str = ": ";
const CONTENT_LENGTH: &str = "Content-Length";

pub async fn send_response_to_stream(
    stream: Box<TcpStream>,
    response: &mut Response,
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

#[cfg(test)]
mod tests {
    use crate::nino_functions::normalize_path;

    #[test]
    fn test_normalize_path() {
        assert_eq!(
            normalize_path(String::from("///////remove//duplicate/////slashes")),
            String::from("remove/duplicate/slashes"),
        );
        assert_eq!(
            normalize_path(String::from("/remove/leading/slash")),
            String::from("remove/leading/slash")
        );
        assert_eq!(
            normalize_path(String::from("this/one/should/not/change")),
            String::from("this/one/should/not/change"),
        );
        assert_eq!(
            normalize_path(String::from("/this/one_____should/not.......duplicate")),
            String::from("this/one_should/not.duplicate"),
        );
        assert_eq!(
            normalize_path(String::from("////...../.exploits/.should.get.normalized/")),
            String::from("exploits/should.get.normalized"),
        );
        assert_eq!(normalize_path(String::from("")), String::from("/"),);
    }
}

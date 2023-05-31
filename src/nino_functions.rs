use async_std::net::TcpStream;
use http_types::Response;

use crate::nino_constants;

/// Get the postgres connection string from the
/// program parameters or system environment variable (in dat order of existance).
/// the name of the program is used as name on the environment parameter (ex nino)
pub fn get_connection_string() -> Result<String, String> {
    use std::env;

    let mut args_iter = env::args().into_iter();
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
        if (b >= 'a' && b <= 'z') || is_special_char {
            if (b == prev) && is_special_char {
                //do not duplicate these chars
            } else if prev == '/' && b == '.' {
                //do not allow adding relative paths
            } else {
                result.push(b);
                prev = b;
            }
        }
    }
    while result.len() > 0 && (*result.get(result.len() - 1).unwrap() == '/') {
        result.pop();
    }
    while result.len() > 0 && (*result.get(0).unwrap() == '/') {
        result.remove(0);
    }
    if result.len() == 0 {
        result.push('/');
    }
    result.into_iter().collect()
}

pub async fn send_response_to_stream(
    stream: &mut TcpStream,
    response: &mut Response,
) -> Result<(), String> {
    const HTTP: &str = "HTTP/1.1";
    const CRLF: &str = "\r\n";
    const SEPARATOR: &str = ": ";
    //const CONTENT_TYPE: &str = "Content-Type";
    const CONTENT_LENGTH: &str = "Content-Length";

    let body = match response.body_bytes().await {
        Ok(v) => v,
        Err(error) => {
            let err = error.to_string();
            return match stream.shutdown(std::net::Shutdown::Both) {
                Ok(_) => Err(err),
                Err(error) => Err(format!("{}\n{}", err, error.to_string())),
            };
        }
    };

    if response.header(CONTENT_LENGTH).is_none() {
        response.insert_header(CONTENT_LENGTH, format!("{}", body.len()));
    }

    let mut header_string = String::with_capacity(1024);
    //write status
    header_string.push_str(&HTTP);
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

    eprintln!("header:\n{}", header_string.to_string());

    //write http
    {
        let mut http_bytes = header_string.as_bytes();
        match async_std::io::copy(&mut http_bytes, &mut stream.clone()).await {
            Ok(_bytes_written) => {}
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error.to_string());
            }
        }
    }

    //write body
    {
        match async_std::io::copy(&mut body.as_slice(), &mut stream.clone()).await {
            Ok(_bytes_written) => {}
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error.to_string());
            }
        }
    }
    //close socket
    if let Err(error) = stream.shutdown(std::net::Shutdown::Both) {
        eprintln!("ERROR {}:{}:{}", file!(), line!(), error.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::nino_functions::normalize_path;

    #[test]
    fn test_normalize_path() {
        assert_eq!(
            normalize_path(String::from_str("///////remove//duplicate/////slashes").unwrap()),
            String::from_str("remove/duplicate/slashes").unwrap(),
        );
        assert_eq!(
            normalize_path(String::from_str("/remove/leading/slash").unwrap()),
            String::from_str("remove/leading/slash").unwrap()
        );
        assert_eq!(
            normalize_path(String::from_str("this/one/should/not/change").unwrap()),
            String::from_str("this/one/should/not/change").unwrap(),
        );
        assert_eq!(
            normalize_path(String::from_str("/this/one_____should/not.......duplicate").unwrap()),
            String::from_str("this/one_should/not.duplicate").unwrap(),
        );
        assert_eq!(
            normalize_path(
                String::from_str("////...../.exploits/.should.get.normalized/").unwrap()
            ),
            String::from_str("exploits/should.get.normalized").unwrap(),
        );
        assert_eq!(
            normalize_path(String::from_str("").unwrap()),
            String::from_str("/").unwrap(),
        );
    }
}

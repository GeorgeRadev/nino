use crate::nino_constants;
use async_std::{io::WriteExt, net::TcpStream};
use bcrypt::{hash, verify, DEFAULT_COST};
use deno_runtime::deno_core::anyhow::Error;
use hmac::{digest::KeyInit, Hmac};
use http_types::Response;
use jwt::{SignWithKey, VerifyWithKey};
use sha2::Sha256;
use std::collections::HashMap;

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

pub async fn send_request_to_stream(
    response_in: reqwest::Response,
    mut stream_out: Box<TcpStream>,
) -> Result<(), Error> {
    //write status
    let mut header_string = String::with_capacity(1024);
    header_string.push_str(HTTP);
    header_string.push(' ');
    header_string.push_str(&format!("{}", response_in.status()));
    header_string.push(' ');
    let canonical_reason = response_in.status().canonical_reason();
    let canonical_reason = canonical_reason.ok_or(Error::msg(format!(
        "cannot resolve status to canonical reason:{}",
        response_in.status()
    )))?;
    header_string.push_str(canonical_reason);
    header_string.push_str(CRLF);

    // write header
    for (header_key, header_value) in response_in.headers() {
        header_string.push_str(header_key.as_str());
        header_string.push_str(SEPARATOR);
        header_string.push_str(header_value.to_str()?);
        header_string.push_str(CRLF);
    }

    //write separtor
    header_string.push_str(CRLF);

    //copy stream
    {
        let mut http_bytes = header_string.as_bytes();
        async_std::io::copy(&mut http_bytes, &mut stream_out.clone()).await?;
        let bytes = response_in.bytes().await?;
        stream_out.write(&bytes).await?;
        stream_out.flush().await?;
    }

    //close socket - always
    if stream_out.shutdown(std::net::Shutdown::Both).is_err() {
        // stream already closed
    }
    Ok(())
}

pub fn password_hash(password: &str) -> Result<String, Error> {
    let hash = match hash(password, DEFAULT_COST) {
        Ok(hash) => hash,
        Err(error) => {
            eprintln!("{}", error);
            return Err(Error::msg(error));
        }
    };
    Ok(hash)
}

pub fn password_verify(password: &str, hash: &str) -> Result<bool, Error> {
    match verify(password, hash) {
        Ok(matched) => Ok(matched),
        Err(error) => Err(Error::msg(error)),
    }
}

pub fn jwt_from_map(secret: &str, map: HashMap<String, String>) -> Result<String, Error> {
    let key: Hmac<Sha256> = Hmac::new_from_slice(secret.as_bytes())?;
    let jwt = map.sign_with_key(&key)?;
    Ok(jwt)
}

pub fn jwt_to_map(secret: &str, jwt: &str) -> Result<HashMap<String, String>, Error> {
    let key: Hmac<Sha256> = Hmac::new_from_slice(secret.as_bytes())?;
    let map_decoded: HashMap<String, String> = jwt.verify_with_key(&key)?;
    Ok(map_decoded)
}

#[cfg(test)]
mod tests {
    use crate::nino_functions::{
        jwt_from_map, jwt_to_map, normalize_path, password_hash, password_verify,
    };
    use std::collections::HashMap;

    #[test]
    fn test_jwt_hashing() {
        let secret = String::from("nino");
        let mut map: HashMap<String, String> = HashMap::new();
        map.insert("key".to_owned(), "value".to_owned());
        let jwt = jwt_from_map(&secret, map).unwrap();
        assert!(!jwt.is_empty());
        let map_decoded = jwt_to_map(&secret, &jwt).unwrap();
        assert!("value" == map_decoded.get("key").unwrap());
    }

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

    #[test]
    fn test_password_hashing() {
        let password = String::from("p@ssw0rd");
        let hash = password_hash(&password).unwrap();
        assert!(password_verify(&password, &hash).unwrap());
        assert!(!password_verify("test", &hash).unwrap());
    }

    #[test]
    fn test_print_password_hash() {
        let password = String::from("admin");
        let hash = password_hash(&password).unwrap();
        println!("password: {}", hash);
    }
}

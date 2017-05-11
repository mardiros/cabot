use std::num::ParseIntError;

use super::results::{CabotResult, CabotError};


#[derive(Debug)]
pub struct Response {
    http_version: String,
    status_code: usize,
    status_line: String,
    headers: Vec<String>,
    body: Option<Vec<u8>>,
}


impl Response {
    fn new(http_version:String,
           status_code: usize,
           status_line: String,
           headers: Vec<String>,
           body: Option<Vec<u8>>)
           -> Response {
        Response {
            http_version: http_version,
            status_code: status_code,
            status_line: status_line,
            headers: headers,
            body: body,
        }
    }

    pub fn http_version(&self) -> &str {
        self.http_version.as_str()
    }

    pub fn status_code(&self) -> usize {
        self.status_code
    }

    pub fn status_line(&self) -> &str {
        self.status_line.as_str()
    }

    pub fn headers(&self) -> Vec<&str> {
        let headers: Vec<&str> = self.headers.iter().map(|s| s.as_ref()).collect();
        headers
    }

    pub fn body(&self) -> Option<&[u8]> {
        match self.body {
            None => None,
            Some(ref body) => {
                Some(body.as_slice())
            }
        }
    }
    
    pub fn body_as_string(&self) -> CabotResult<String> {
        let body = match self.body {
            None => "".to_owned(),
            Some(ref body) => {
                let mut body_vec: Vec<u8> = Vec::new();
                body_vec.extend_from_slice(body);
                let body_str = String::from_utf8(body_vec);
                if body_str.is_err() {
                    return Err(CabotError::EncodingError(format!("Cannot decode utf8: {}", body_str.unwrap_err())))
                }
                body_str.unwrap()
            }
        };
        Ok(body)
    }
}


#[derive(Debug)]
pub struct ResponseBuilder {
    status_line: Option<String>,
    headers: Vec<String>,
    body: Option<Vec<u8>>,
}

impl ResponseBuilder {
    pub fn new() -> Self {
        ResponseBuilder {
            status_line: None,
            headers: Vec::new(),
            body: None,
        }
    }

    pub fn set_status_line(mut self, status_line: &str) -> Self {
        self.status_line = Some(status_line.to_string());
        self
    }

    pub fn add_header(mut self, header: &str) -> Self {
        self.headers.push(header.to_owned());
        self
    }

    pub fn set_body(mut self, buf: &[u8]) -> Self {
        let mut body = Vec::with_capacity(buf.len()); 
        body.extend_from_slice(buf);
        self.body = Some(body);
        self
    }

    pub fn build(&self) -> CabotResult<Response> {
        if self.status_line.is_none() {
            return Err(CabotError::HttpResponseParseError("No Status Line".to_owned()));
        }

        let status_line = self.status_line.as_ref().unwrap();
        let mut vec_status_line: Vec<&str> = status_line.splitn(3, " ").collect();

        if vec_status_line.len() != 3 {
            return Err(CabotError::HttpResponseParseError(format!("Malformed Status Line: {}", status_line)));
        }

        let http_version = vec_status_line.remove(0);
        if !http_version.starts_with("HTTP/") {
            return Err(CabotError::HttpResponseParseError(format!("Unkown Protocol in Status Line: {}", status_line)));
        }

        let status_code = vec_status_line.get(0).unwrap();
        let status_code: Result<usize, ParseIntError> = status_code.parse();
        if status_code.is_err() {
            return Err(CabotError::HttpResponseParseError(format!("Malformed status code: {}", status_line)));
        }
        let status_code = status_code.unwrap();
        let status_line = vec_status_line.as_slice().join(" ");

        Ok(Response::new(http_version.to_owned(),
                         status_code,
                         status_line,
                         self.headers.to_owned(),
                         self.body.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_ok() {
        let response = Response::new(
            "HTTP/1.1".to_owned(),
            200,
            "200 Ok".to_owned(),
            vec!["Content-Type: application/json".to_owned()],
            Some(vec![123, 125]));

        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.status_line(), "200 Ok");
        assert_eq!(response.headers(), &["Content-Type: application/json"]);
        let body: &[u8] = &[123, 125];
        assert_eq!(response.body(), Some(body));
        assert_eq!(response.body_as_string().unwrap(), "{}");
    }

    #[test]
    fn test_response_ok_no_body() {
        let response = Response::new(
            "HTTP/1.1".to_owned(),
            204,
            "204 No Content".to_owned(),
            vec![],
            None);

        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 204);
        assert_eq!(response.status_line(), "204 No Content");
        let headers: &[&str] = &[];
        assert_eq!(response.headers(), headers);
        assert_eq!(response.body(), None);
        assert_eq!(response.body_as_string().unwrap(), "".to_string());
    }

    #[test]
    fn test_build_response_ok() {
        let response = ResponseBuilder::new()
            .set_status_line("HTTP/1.1 200 Ok")
            .add_header("Content-Type: application/json")
            .set_body(&[123, 125])
            .build()
            .unwrap();

        assert_eq!(response.http_version(), "HTTP/1.1");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.status_line(), "200 Ok");
        assert_eq!(response.headers(), &["Content-Type: application/json"]);
        let body: &[u8] = &[123, 125];
        assert_eq!(response.body(), Some(body));
        assert_eq!(response.body_as_string().unwrap(), "{}");
    }


}
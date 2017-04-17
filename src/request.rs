use url::{self, Url};

use results::{CabotResult, CabotError};


pub struct Request {
    host: String,
    port: u16,
    authority: String,
    is_domain: bool,
    scheme: String,
    http_method: String,
    request_uri: String,
    http_version: String,
    headers: Vec<String>,
    body: Option<String>,
}

impl Request {
    fn new(host: String,
           port: u16,
           authority: String,
           is_domain: bool,
           scheme: String,
           http_method: String,
           request_uri: String,
           http_version: String,
           headers: Vec<String>,
           body: Option<String>)
           -> Request {
        Request {
            host: host,
            port: port,
            authority: authority,
            is_domain: is_domain,
            scheme: scheme,
            http_method: http_method,
            request_uri: request_uri,
            http_version: http_version,
            headers: headers,
            body: body,
        }
    }
    pub fn http_method(&self) -> &str {
        self.http_method.as_str()
    }

    pub fn body(&self) -> Option<&str> {
        match self.body {
            Some(ref payload) => Some(payload.as_str()),
            None => None,
        }
    }
    pub fn http_version(&self) -> &str {
        self.http_version.as_str()
    }

    pub fn host(&self) -> &str {
        self.host.as_str()
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn authority(&self) -> &str {
        self.authority.as_str()
    }

    pub fn scheme(&self) -> &str {
        self.scheme.as_str()
    }

    pub fn request_uri(&self) -> &str {
        self.request_uri.as_str()
    }

    pub fn to_string(&self) -> String {
        let mut resp = format!("{} {} {}\r\n",
                               self.http_method(),
                               self.request_uri(),
                               self.http_version());
        if self.headers.len() > 0 {
            resp.push_str(self.headers.as_slice().join("\r\n").as_str());
            resp.push_str("\r\n");
        }
        if self.is_domain {
            resp.push_str(format!("Host: {}\r\n", self.host()).as_str());
        }
        resp.push_str("Connection: close\r\n");
        if let Some(payload) = self.body() {
            resp.push_str(format!("Content-Length: {}\r\n", payload.len()).as_str());
            resp.push_str("\r\n");
            resp.push_str(payload);
        } else {
            resp.push_str("\r\n");
        }
        resp
    }
}


pub struct RequestBuilder {
    http_method: String,
    url: Result<Url, url::ParseError>,
    http_version: String,
    headers: Vec<String>,
    body: Option<String>,
}

impl RequestBuilder {
    pub fn new(url: &str) -> Self {
        let url = url.parse::<Url>();
        RequestBuilder {
            http_method: "GET".to_owned(),
            url: url,
            http_version: "HTTP/1.1".to_owned(),
            headers: Vec::new(),
            body: None,
        }
    }

    pub fn set_url(mut self, url: &str) -> Self {
        self.url = url.parse::<Url>();
        self
    }

    pub fn set_http_method(mut self, http_method: &str) -> Self {
        self.http_method = http_method.to_owned();
        self
    }

    pub fn set_http_version(mut self, http_version: &str) -> Self {
        self.http_version = http_version.to_owned();
        self
    }

    pub fn add_header(mut self, header: &str) -> Self {
        self.headers.push(header.to_owned());
        self
    }

    pub fn set_body(mut self, body: &str) -> Self {
        self.body = Some(body.to_owned());
        self
    }

    pub fn build(&self) -> CabotResult<Request> {
        if let Err(ref err) = self.url {
            return Err(CabotError::UrlParseError(err.clone()));
        }
        let url = self.url.as_ref().unwrap().clone();

        let host = url.host_str();
        if host.is_none() {
            return Err(CabotError::OpaqueUrlError("Unable to find host".to_string()));
        }
        let host = host.unwrap();

        let port = url.port_or_known_default();
        if port.is_none() {
            return Err(CabotError::OpaqueUrlError("Unable to determine a port".to_string()));
        }
        let port = port.unwrap();

        let query = url.query();
        let mut request_uri = url.path().to_owned();
        if let Some(querystring) = query {
            request_uri.push_str("?");
            request_uri.push_str(querystring);
        }
        let mut is_domain = true;
        if url.domain().is_none() {
            is_domain = false;
        }

        Ok(Request::new(host.to_owned(),
                        port,
                        format!("{}:{}", host, port),
                        is_domain,
                        url.scheme().to_owned(),
                        self.http_method.clone(),
                        request_uri,
                        self.http_version.clone(),
                        self.headers.clone(),
                        match self.body {
                            Some(ref body) => Some(body.clone()),
                            None => None,
                        }))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_request_to_string() {
        let request = Request::new("127.0.0.1".to_owned(),
                                   80,
                                   "127.0.0.1:80".to_owned(),
                                   false,
                                   "http".to_owned(),
                                   "GET".to_owned(),
                                   "/path?query".to_owned(),
                                   "HTTP/1.1".to_owned(),
                                   Vec::new(),
                                   None);
        let attempt = "GET /path?query HTTP/1.1\r\nConnection: close\r\n\r\n";
        assert_eq!(request.to_string(), attempt);
    }

    #[test]
    fn test_get_request_wiht_host_to_string() {
        let request = Request::new("localhost".to_owned(),
                                   80,
                                   "localhost:80".to_owned(),
                                   true,
                                   "http".to_owned(),
                                   "GET".to_owned(),
                                   "/path?query".to_owned(),
                                   "HTTP/1.1".to_owned(),
                                   Vec::new(),
                                   None);
        let attempt = "GET /path?query HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
        assert_eq!(request.to_string(), attempt);
    }

    #[test]
    fn test_get_request_with_headers_to_string() {
        let request = Request::new("localhost".to_owned(),
                                   80,
                                   "localhost:80".to_owned(),
                                   true,
                                   "http".to_owned(),
                                   "GET".to_owned(),
                                   "/path?query".to_owned(),
                                   "HTTP/1.1".to_owned(),
                                   vec!["Accept-Language: fr".to_owned(),
                                        "Accept-Encoding: gzip".to_owned()],
                                   None);
        let attempt = "GET /path?query HTTP/1.1\r\nAccept-Language: fr\r\nAccept-Encoding: \
                       gzip\r\nHost: localhost\r\nConnection: close\r\n\r\n";
        assert_eq!(request.to_string(), attempt);
    }

    #[test]
    fn test_post_request_with_headers_to_string() {
        let request = Request::new("localhost".to_owned(),
                                   80,
                                   "localhost:80".to_owned(),
                                   true,
                                   "http".to_owned(),
                                   "POST".to_owned(),
                                   "/".to_owned(),
                                   "HTTP/1.1".to_owned(),
                                   vec!["Accept-Language: fr".to_owned(),
                                        "Content-Type: application/json".to_owned()],
                                   Some("{}".to_owned()));
        let attempt = "POST / HTTP/1.1\r\nAccept-Language: fr\r\nContent-Type: \
                       application/json\r\nHost: localhost\r\nConnection: \
                       close\r\nContent-Length: 2\r\n\r\n{}";
        assert_eq!(request.to_string(), attempt);
    }

    #[test]
    fn test_request_builder_simple() {
        let request = RequestBuilder::new("http://localhost/")
            .build()
            .unwrap();
        assert_eq!(request.host(), "localhost".to_string());
        assert_eq!(request.scheme(), "http".to_string());
        assert_eq!(request.body, None);
        assert_eq!(request.http_method(), "GET".to_string());
        assert_eq!(request.http_version(), "HTTP/1.1".to_string());
        let headers: Vec<String> = Vec::new();
        assert_eq!(request.headers, headers);
    }

    #[test]
    fn test_request_builder_complete() {
        let builder = RequestBuilder::new("http://localhost/")
            .set_http_method("POST")
            .set_http_version("HTTP/1.0")
            .add_header("Content-Type: application/json")
            .set_body("{}");
        let request = builder.build().unwrap();
        assert_eq!(request.host(), "localhost".to_string());
        assert_eq!(request.body, Some("{}".to_string()));
        assert_eq!(request.scheme(), "http".to_string());
        assert_eq!(request.http_method(), "POST".to_string());
        assert_eq!(request.request_uri(), "/");
        assert_eq!(request.http_version(), "HTTP/1.0".to_string());
        assert_eq!(request.headers,
                   vec!["Content-Type: application/json".to_string()]);

        let builder = builder.set_url("http://[::1]/path");
        let request = builder.build().unwrap();
        assert_eq!(request.host(), "[::1]".to_string());
        assert_eq!(request.request_uri(), "/path");
        assert_eq!(request.body, Some("{}".to_string()));
        assert_eq!(request.scheme(), "http".to_string());
        assert_eq!(request.http_method(), "POST".to_string());
        assert_eq!(request.http_version(), "HTTP/1.0".to_string());
        assert_eq!(request.headers,
                   vec!["Content-Type: application/json".to_string()]);

        let builder = builder.set_url("not_an_url");
        let err = builder.build();
        assert!(err.is_err());
    }

}

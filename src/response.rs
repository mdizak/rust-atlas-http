
use super::{HttpRequest, HttpResponse, HttpHeaders};
use crate::error::{Error, InvalidResponseError};

impl HttpResponse { 

    /// Instantiate response with minimal properties
    pub fn new(status: &u16, headers: &Vec<String>, body: &String) -> Self {
        Self::new_full(&status, &HttpHeaders::from_vec(&headers), &body, &"1.1".to_string(), &"".to_string())
    } 

    /// Instantiate new response with all properties
    pub fn new_full(status: &u16, headers: &HttpHeaders, body: &String, version: &String, reason: &String) -> Self {

        Self {
            version: version.clone(),
            status_code: *status,
            reason: reason.clone(),
            headers: headers.clone(),
            body: body.trim().trim_end_matches("0").to_string()
        }
    }

    /// Get protocol version
    pub fn version(&self) ->String {
        self.version.clone()
    }

    /// Get HTTP status code
    pub fn status_code(&self) -> u16 {
        self.status_code
    }

    /// Get status reason message
    pub fn reason(&self) -> String {
        self.reason.clone()
    }

    /// Get http headers
    pub fn headers(&self) -> HttpHeaders {
        self.headers.clone()
    }

    /// Get body of response
    pub fn body(&self) -> String {
        self.body.clone()
    }

    /// Get the raw response including headers and body
    pub fn raw(&self) -> String {

        let headers_str = self.headers.all().iter().map(|(key, value)| {
            format!("{}: {}", key, value.join("; "))
        }).collect::<Vec<String>>().join("\r\n");

        let res = format!("HTTP/{} {} {}\r\n{}\n\n{}\n\n", self.version, self.status_code, self.reason, &headers_str, self.body);
        res.to_string()
    }

}




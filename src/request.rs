
use super::{HttpRequest, HttpHeaders, HttpBody};
use std::collections::HashMap;

impl HttpRequest {

    pub fn new(method: &str, url: &str, headers: &Vec<&str>, body: &HttpBody) ->Self {

        Self {
            method: method.to_uppercase().to_string(),
            url: url.to_string(),
            headers: HttpHeaders::from_vec(&headers.iter().map(|s| s.to_string()).collect()),
            body: body.clone()
        }
    }

}


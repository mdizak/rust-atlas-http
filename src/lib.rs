#![allow(warnings)]
pub mod body;
pub mod client;
pub mod client_builder;
pub mod cookie;
pub mod error;
pub mod headers;
pub mod request;
pub mod response;
pub mod tls_noverify;
pub mod user_agent;

use std::sync::Arc;
use std::collections::HashMap;
use crate::client::ProxyType;

#[derive(Debug, Clone)]
pub struct HttpClient { 
    config: Arc<rustls::ClientConfig>,
    user_agent: Option<String>,
    headers: HttpHeaders,
    follow_location: bool,
    timeout: u64,
    proxy_type: ProxyType,
    proxy_host: String,
    proxy_port: usize,
    proxy_user: String,
    proxy_password: String
}

#[derive(Clone, Debug)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: HttpHeaders,
    pub body: HttpBody
}

#[derive(Clone, Debug)]
pub struct HttpResponse {
    version: String,
    status_code: u16,
    reason: String,
    headers: HttpHeaders,
    body: String
}

#[derive(Clone, Debug)]
pub struct HttpBody {
    is_form_post: bool,
    params: HashMap<String, String>,
    raw: Vec<u8>,
    boundary: String,
    files: HashMap<String, String>
}

#[derive(Clone, Debug)]
pub struct HttpHeaders {
    pairs: HashMap<String, Vec<String>>,
    lower_map: HashMap<String, String>
}

#[derive(Clone, Debug)]
pub struct HttpCookie { }



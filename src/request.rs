use super::{HttpBody, HttpClientConfig, HttpHeaders, ProxyType};
use crate::error::Error;
use url::Url;

#[derive(Clone, Debug)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: HttpHeaders,
    pub body: HttpBody,
}

impl HttpRequest {
    pub fn new(method: &str, url: &str, headers: &Vec<&str>, body: &HttpBody) -> Self {
        Self {
            method: method.to_uppercase().to_string(),
            url: url.to_string(),
            headers: HttpHeaders::from_vec(&headers.iter().map(|s| s.to_string()).collect()),
            body: body.clone(),
        }
    }

    // Validate URL and scheme
    pub fn prepare(&self, config: &HttpClientConfig) -> Result<(Url, u16, Vec<u8>), Error> {
        // Parse url
        let uri = match Url::parse(&self.url) {
            Ok(r) => r,
            Err(_err) => {
                return Err(Error::InvalidUri(self.url.clone()));
            }
        };

        // Check scheme
        if uri.scheme() != "http" && uri.scheme() != "https" {
            return Err(Error::ProtoNotSupported(uri.scheme().to_string()));
        }

        // Get port
        let mut _port: u16 = 0;
        if uri.port().is_none() && uri.scheme() == "https" {
            _port = 443;
        } else if uri.port().is_none() && uri.scheme() == "http" {
            _port = 80;
        } else {
            _port = uri.port().unwrap();
        }

        // Generate message
        let message = self.generate_raw(config, &uri);

        Ok((uri, _port, message))
    }

    /// Generate raw HTTP message to be sent
    fn generate_raw(&self, config: &HttpClientConfig, uri: &Url) -> Vec<u8> {
        // Get target
        let mut target = uri.path().to_string();
        if config.proxy_type != ProxyType::None {
            target = format!(
                "{}://{}{}",
                uri.scheme(),
                uri.host_str().unwrap(),
                uri.path()
            );
        }

        let mut lines = vec![
            format!("{} {} HTTP/1.1", &self.method, target),
            format!("Host: {}", uri.host_str().unwrap()),
        ];

        if let Some(ua) = &config.user_agent {
            lines.push(format!("User-Agent: {}", ua));
        }

        // HTTP client headers
        for (key, value) in config.headers.all().iter() {
            lines.push(format!("{}: {}", key, value.join("; ")));
        }

        // Cookie header
        if let Some(cookie_hdr) = config.cookie.get_http_header(uri) {
            lines.push(format!("Cookie: {}", cookie_hdr));
        }

        // POST headers
        if !self.body.files().is_empty() && !self.headers.has_lower("content-type") {
            lines.push(format!(
                "Content-type: multipart/form-data; boundary={}",
                self.body.boundary()
            ));
        } else if self.body.is_form_post() && !self.headers.has_lower("content-type") {
            lines.push("Content-type: application/x-www-form-urlencoded".to_string());
        }

        // Format post body, if needed
        let mut post_body: Vec<u8> = Vec::new();
        if self.body.is_form_post() {
            post_body = self.body.format();
            lines.push(format!("Content-length: {}", post_body.len()));
        }

        // HTTP request headers
        for (key, value) in self.headers.all().iter() {
            lines.push(format!("{}: {}", key, value.join("; ")));
        }
        lines.push("\r\n".to_string());

        // Add body
        let mut message = lines.join("\r\n").as_bytes().to_vec();
        message.extend(post_body);
        message.extend_from_slice("\r\n".as_bytes());
println!("Request:\n\n{}\n\n", String::from_utf8_lossy(&message));
        message
    }
}

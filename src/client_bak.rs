#![allow(warnings)]
use std::io::{BufRead, BufReader, Write, Read};
use std::fs::File;
use std::path::Path;
use std::net::{TcpStream, SocketAddr};
use std::net::ToSocketAddrs;
use std::time::Duration;
use std::sync::Arc;
use url::Url;
use rustls::{RootCertStore, ClientConfig};
use rustls::pki_types::ServerName;
use webpki_roots::TLS_SERVER_ROOTS;
use super::{HttpClient, HttpRequest, HttpResponse, HttpBody, HttpHeaders};
use crate::error::{Error, InvalidResponseError, InvalidFirstLineError, FileNotCreatedError};
use crate::{user_agent, tls_noverify};
use crate::client_builder::HttpClientBuilder;
use std::io;

pub trait ClientStream {
    fn write(&mut self, data: &[u8]) -> Result<(), Error>;
    fn get_stream(&self) -> Box<dyn Read>;
}

pub struct HttpClientStream { 
    pub stream: TcpStream 
}

impl ClientStream for HttpClientStream { 
    fn write(&mut self, data: &[u8]) -> Result<(), Error> {
        match self.stream.write_all(&data) {
            Ok(_) => { },
            Err(e) => { return Err( Error::NoWrite(e.to_string()) ); }
        };
        Ok(())
    }

    fn get_stream(&self) -> Box<dyn Read> {
        let cloned_stream = self.stream.try_clone().unwrap();
        Box::new(cloned_stream)
    }
}

pub struct TlsClientStream<'a, 'b> { 
    pub stream: rustls::Stream<'a, rustls::ClientConnection, TcpStream>,
    tcp_stream: &'b mut TcpStream,
    client_connection: &'b mut rustls::ClientConnection
}

impl <'a, 'b>ClientStream for TlsClientStream<'a, 'b> { 

    fn write(&mut self, data: &[u8]) -> Result<(), Error> {
        self.stream.write_all(&data).unwrap();
        Ok(())
    }

    fn get_stream(&self) -> Box<dyn Read> {
        //let cloned_stream = self.stream.clone();
        //Box::new( BufReader::new(cloned_stream) )
        Box::new(BufReader::new(self.stream))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProxyType {
    None,


    HTTP,
    SOCKS5
}

impl HttpClient {

    pub fn new() -> Self {

        // Initialize root store
        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        // Create config
        let mut config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        Self { 
            config: Arc::new(config),
            user_agent: None,
            headers: HttpHeaders::from_vec(&vec!["Connection: close".to_string()]),
            follow_location: false,
            timeout: 5,
            proxy_type: ProxyType::None,
            proxy_host: String::new(),
            proxy_port: 0,
            proxy_user: String::new(),
            proxy_password: String::new()
        }

    }

    /// Instantiate HTTP client builder
    pub fn builder() -> HttpClientBuilder {
        HttpClientBuilder::new()
    }

    /// Send HTTP request, and return response
    pub fn send(&self, req: &HttpRequest) -> Result<HttpResponse, Error> {
        self.send_request(&req, &String::new())
    }

    /// Download a file
    pub fn download(&self, url: &str, dest_file: &str) -> Result<HttpResponse, Error> {
        let req = HttpRequest::new("GET", &url, &vec![], &HttpBody::empty());
        self.send_request(&req, &dest_file.to_string())
    }

    /// Send GET request
    pub fn get(&self, url: &str) -> Result<HttpResponse, Error> {
        let req = HttpRequest::new("GET", &url, &Vec::new(), &HttpBody::empty());
    self.send_request(&req, &String::new())
    }

    /// Send POST request
    pub fn post(&self, url: &str, body: &HttpBody) -> Result<HttpResponse, Error> {
        let req = HttpRequest::new("POST", &url, &Vec::new(), &body);
    self.send_request(&req, &String::new())
    }

    /// Send PUT request
    pub fn put(&self, url: &str, data: &Vec<u8>) -> Result<HttpResponse, Error> {
        let req = HttpRequest::new("PUT", &url, &Vec::new(), &HttpBody::from_raw(&data));
    self.send_request(&req, &String::new())
    }

    /// Send DELETE request
    pub fn delete(&self, url: &str) -> Result<HttpResponse, Error> {
        let req = HttpRequest::new("DELETE", &url, &Vec::new(), &HttpBody::empty());
    self.send_request(&req, &String::new())
    }

    /// Send OPTIONS request
    pub fn options(&self, url: &str) -> Result<HttpResponse, Error> {
        let req = HttpRequest::new("OPTIONS", &url, &Vec::new(), &HttpBody::empty());
    self.send_request(&req, &String::new())
    }

    /// Send HEAD request
    pub fn head(&self, url: &str) -> Result<HttpResponse, Error> {
        let req = HttpRequest::new("HEAD", &url, &Vec::new(), &HttpBody::empty());
    self.send_request(&req, &String::new())
    }

    // Send request, used internally by the other methods.
    fn send_request(&self, req: &HttpRequest, dest_file: &String) -> Result<HttpResponse, Error> {

        // Prepare uri and http message
        let (uri, port, message) = self.prepare(&req)?;

        // Connect
        let mut stream = self.connect(&uri, &port)?;

        // Send http request
        stream.write(&message);
        let mut reader = BufReader::new(stream.get_stream());

        // Read header
        let (version, status, reason, header_lines) = self.read_header(&mut reader, &req)?;

        // Get body
        let mut body = String::new();
        if dest_file.is_empty() {
            reader.read_to_string(&mut body);
        }

        // Get response
        let mut res = HttpResponse::new_full(&status, &HttpHeaders::from_vec(&header_lines), &body, &version, &reason);

        // Check follow location
        if self.follow_location && res.headers().has_lower("location") {
            let redirect_req = HttpRequest::new("GET", res.headers().get_lower("location").unwrap().as_str(), &vec![], &HttpBody::empty());
            res = self.send_request(&redirect_req, &dest_file)?;
        }

        // Return if not downloading a file
        if dest_file.is_empty() {
            return Ok(res);
        }

        // Save output file
        let dest_path = Path::new(&dest_file);
        let mut fh = match File::create(&dest_path) {
            Ok(r) => r,
            Err(e) => { return Err ( Error::FileNotCreated( FileNotCreatedError { filename: dest_file.to_string(), error: e.to_string() }) ); }
        };

        // Save file
        let mut buffer = [0u8; 2048];
        loop {
            let bytes_read = match reader.read(&mut buffer) {
                Ok(r) => r,
                Err(e) => { return Err( Error::NoRead( InvalidResponseError { request: req.clone(), response: e.to_string() }) ); }
            };

            if bytes_read == 0 {
                break;
            }
            fh.write(&buffer).unwrap();
        }

        Ok(res)
    }

    // Validate URL and scheme
    fn prepare(&self, req: &HttpRequest) -> Result<(Url, u16, Vec<u8>), Error> {

        // Parse url
        let uri = match Url::parse(&req.url) {
            Ok(r) => r,
            Err(err) => { return Err( Error::InvalidUri(req.url.clone()) ); }
        };

        // Check scheme
        if uri.scheme() != "http" && uri.scheme() != "https" {
            return Err ( Error::ProtoNotSupported(uri.scheme().to_string()) );
        }

        // Get port
        let mut port: u16 = 0;
        if uri.port() == None && uri.scheme() == "https" {
            port = 443;
        } else if uri.port() == None && uri.scheme() == "http" {
            port = 80;
        } else {
            port = uri.port().unwrap();
        }

        // Generate message
        let message = self.generate_raw(&req, &uri);

        Ok((uri, port, message))
    }

    // Connect to remote server
    fn connect(&self, uri: &Url, port: &u16) -> Result<Box<dyn ClientStream>, Error> {

        // Prepare uri
        let hostname = if self.proxy_type == ProxyType::HTTP && self.proxy_host != "" {
            format!("{}:{}", self.proxy_host, self.proxy_port)
        } else {
            format!("{}:{}", &uri.host_str().unwrap(), port)
        };
        let mut address = hostname.to_socket_addrs().unwrap();
        let addr = address.next().unwrap();

        // Open tcp stream
        let mut sock = match TcpStream::connect_timeout(&addr, Duration::from_secs(self.timeout)) {
            Ok(r) => r,
            Err(e) => { return Err( Error::NoConnect(hostname.clone()) ); }
        };
        sock.set_nodelay(true).unwrap();

        // Connect over SSL, if needed
        if uri.scheme() == "https" && self.proxy_type == ProxyType::None {
            let dns_name = ServerName::try_from(uri.host_str().unwrap()).unwrap().to_owned();
            let mut conn = rustls::ClientConnection::new(Arc::clone(&self.config), dns_name).unwrap();

            let mut tls_stream = rustls::Stream::new(&mut conn, &mut sock);
            tls_stream.flush().unwrap();

            let mut tmp = TlsClientStream {
                stream: Box::new<tls_stream>,
                tcp_stream: Box::new(&mut sock),
                client_connection: Box::new(conn)
            };

            //tmp.stream = Some ( rustls::Stream::new(&mut tmp.client_connection, &mut tmp.tcp_stream) );
            //tmp.stream.unwrap().flush().unwrap();
            return Ok(Box::new(tmp));
        }

        Ok( Box::new( HttpClientStream { stream: sock } ) )
    }

    /// Will always follow Location headers it encounters
    pub fn follow_location(&mut self) {
        self.follow_location = true;
    }

    // Set timeout limit in seconds
    pub fn timeout(&mut self, seconds: u64) {
        self.timeout = seconds;
    }

    /// Do not verify SSL certificates
    pub fn noverify_ssl(&mut self) {

        // Initialize root store
        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        // Create config
        let mut config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        config.dangerous()
            .set_certificate_verifier(Arc::new(tls_noverify::NoCertificateVerification::new(
                rustls::crypto::ring::default_provider(),
        )));

        self.config = Arc::new(config);
    }

    /// Set base headers to more closely emulate a web browser.
    pub fn browser(&mut self) {

        // Create headers
        self.headers = HttpHeaders::new();
        self.headers.set("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8");
        self.headers.set("Accept-Language", "en-US,en;q=0.5");
        self.headers.set("Accept-Encoding", "identity");
        self.headers.set("Connection", "close");

        // User agent
        if self.user_agent == None {
            self.user_agent = Some(user_agent::random());
        }
        self.follow_location = true;

    }

    /// Send requests over the Tor network.
    pub fn tor(&mut self) {
        self.proxy("127.0.0.1", &9050);
        self.proxy_type(ProxyType::SOCKS5);
    }

    // Send requests through a HTTP / SOCKS5 proxy
    pub fn proxy(&mut self, host: &str, port: &usize) {
        if self.proxy_type == ProxyType::None {
            self.proxy_type = ProxyType::SOCKS5;
        }
        self.proxy_host = host.to_string();
        self.proxy_port = *port;
    }

    // Define authentication for the HTTP / SOCKS5 proxy
    pub fn proxy_auth(&mut self, user: &str, password: &str) {
        self.proxy_user = user.to_string();
        self.proxy_password = password.to_string();
    }

    /// Define whether it's a HTTP or SOCKS5 proxy
    pub fn proxy_type(&mut self, proxy_type: ProxyType) {
        self.proxy_type = proxy_type;
    }

    /// Generate raw HTTP message to be sent
    fn generate_raw(&self, req: &HttpRequest, uri: &Url) -> Vec<u8> {

        let mut lines = vec![
            format!("{} {} HTTP/1.1", &req.method, uri.path()),
            format!("Host: {}", uri.host_str().unwrap())
        ];

        if let Some(ua) = &self.user_agent {
            lines.push(format!("User-Agent: {}", ua));
        }

        // HTTP client headers
        for (key, value) in self.headers.all().iter() {
            lines.push(format!("{}: {}", key, value.join("; ")));
        }

        // POST headers
        if req.body.files().len() > 0 && !req.headers.has_lower("content-type") {
            lines.push(format!("Content-type: multipart/form-data; boundary={}", req.body.boundary()));
        } else if req.body.is_form_post() && !req.headers.has_lower("content-type") {
            lines.push("Content-type: application/x-www-form-urlencoded".to_string());
        }

        // Format post body, if needed
        let mut post_body: Vec<u8> = Vec::new();
        if req.body.is_form_post() {
            post_body = req.body.format();
            lines.push(format!("Content-length: {}", post_body.len()));
        }

        // HTTP request headers
        for (key, value) in req.headers.all().iter() {
            lines.push(format!("{}: {}", key, value.join("; ")));
        }
        lines.push("\r\n".to_string());

        // Add body
        let mut message = lines.join("\r\n").as_bytes().to_vec();
        message.extend(post_body);
        message.extend_from_slice("\r\n".as_bytes());

        message
    }

    /// Read first line and header of response
    fn read_header(&self, reader: &mut BufReader<Box<dyn Read>>, req: &HttpRequest) -> Result<(String, u16, String, Vec<String>), Error> {

        // Get first line
        let mut first_line = String::new();
        match reader.read_line(&mut first_line) {
            Ok(_) => { },
            Err(e) => { return Err( Error::NoRead( InvalidResponseError { request: req.clone(), response: e.to_string() }) ); }
        };

        // Parse first line
        let (version, status, reason) = self.parse_first_line(&first_line, &req)?;

        // Get headers
        let mut headers = Vec::new();
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(_) => { },
                Err(e) => { return Err( Error::NoRead( InvalidResponseError { request: req.clone(), response: e.to_string() }) ); }
            };

            if line.trim().is_empty() {
                break;
            }
            headers.push(line.trim().to_string());
        }

        Ok((version, status, reason, headers))
    }

    /// Parse first line
    fn parse_first_line(&self, first_line: &str, req: &HttpRequest) -> Result<(String, u16, String), Error> {

        // Parse first line
        let mut is_valid = true;
        let parts = first_line.trim_start_matches("HTTP/").split(" ").collect::<Vec<&str>>();
        if !vec!["1.0", "1.1", "2", "3"].contains(&parts[0]) {
            is_valid = false;
        } else if parts[1].len() != 3 || !parts[1].chars().all(|c| c.is_ascii_digit()) {
            is_valid = false;
        }

        if !is_valid {
            let error = InvalidFirstLineError { request: req.clone(), first_line: first_line.to_string() };
            return Err( Error::InvalidFirstLine(error) );
        }

        Ok((parts[0].to_string(), parts[1].parse::<u16>().unwrap(), parts[2].to_string()))
    }

}




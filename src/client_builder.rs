
use crate::HttpClient;
use crate::client::ProxyType;

pub struct HttpClientBuilder { 
    client: HttpClient
}

impl HttpClientBuilder {

    pub fn new() -> Self {
        Self { 
            client: HttpClient::new()
        }
    }

        /// Finish building, and get HTTP client instance
    pub fn build(&mut self) -> HttpClient {
        self.client.clone()
    }

    /// Will always follow Location headers it encounters
    pub fn follow_location(mut self) -> Self {
        self.client.follow_location();
        self
    }

    // Set timeout limit in seconds
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.client.timeout(seconds);
        self
    }

    /// Do not verify SSL certificates
    pub fn noverify_ssl(mut self) -> Self {
        self.client.noverify_ssl();
        self
    }

    /// Set base headers to more closely emulate a web browser.
    pub fn browser(mut self) -> Self {
        self.client.browser();
        self
    }

    /// Send requests over the Tor network.
    pub fn tor(mut self) -> Self {
        self.client.tor();
        self
    }

    // Send requests through a HTTP / SOCKS5 proxy
    pub fn proxy(mut self, host: &str, port: &usize) -> Self {
        self.client.proxy(&host, &port);
        self
    }

    // Define authentication for the HTTP / SOCKS5 proxy
    pub fn proxy_auth(mut self, user: &str, password: &str) -> Self {
        self.client.proxy_auth(&user, &password);
        self
    }

    /// Define whether it's a HTTP or SOCKS5 proxy
    pub fn proxy_type(mut self, proxy_type: ProxyType) -> Self {
        self.client.proxy_type(proxy_type);
        self
    }

}



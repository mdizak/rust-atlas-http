
Right now, create only supports SSL connections on port 443, but doesn't support port 80 or SOCKS5.  Tried to my wit's end, but unable 
to get a uniform connection to work.  Below are two tries I did, one using a 
trait object and another using enum type.  Neither of these compile, but played around with 
tthundreds of tweaks and never any luck.

Issues were always the same.  Main one came from creating the TLS stream with the line:

> let mut tls_stream = rustls::Stream::new(&mut conn, &mut sock);

Borrowing mutable references to 'conn' and 'sock' caused problems trying to get it into the struct of the trait object, 
or outside of the function even.  Can't borrow locally created variable, using BOx::new() to move to heap didn't really work, 
mismatched data type errors sometimes, and so on.

If anyone could be kind enough to somehow update the below ro make an implmenetation work where I can connect to either 
TCP on port 80 over TLS on port 443, plus read to and write from them such as I'm trying below, it would 
be greatly appreciated.  If I can see how that's done, I'll be able to implement SOCKS5 without issue.

===========================================

pub enum ConnectionStream<'a> {
    Tcp(TcpStream),
    Tls(rustls::Stream<'a, rustls::ClientConnection, TcpStream>)
}

impl ConnectionStream<'_> {

    pub fn connect(http: &HttpClient, uri: &Url, port: &u16) -> Result<Self, Error> {

        // Prepare uri
        let hostname = if http.proxy_type == ProxyType::HTTP && http.proxy_host != "" {
            format!("{}:{}", http.proxy_host, http.proxy_port)
        } else {
            format!("{}:{}", &uri.host_str().unwrap(), port)
        };
        let mut address = hostname.to_socket_addrs().unwrap();
        let addr = address.next().unwrap();

        // Open tcp stream
        let mut sock = match TcpStream::connect_timeout(&addr, Duration::from_secs(http.timeout)) {
            Ok(r) => r,
            Err(e) => { return Err( Error::NoConnect(hostname.clone()) ); }
        };
        sock.set_nodelay(true).unwrap();

        // Connect as needed
        if uri.scheme() == "https" && http.proxy_type == ProxyType::None {
            let dns_name = ServerName::try_from(uri.host_str().unwrap()).unwrap().to_owned();
            let mut conn = rustls::ClientConnection::new(Arc::clone(&http.config), dns_name).unwrap();

            let mut tls_stream = rustls::Stream::new(&mut conn, &mut sock);
            tls_stream.flush().unwrap();
            Ok(ConnectionStream::Tls(tls_stream))
        } else {
            Ok(ConnectionStream::Tcp(sock))
        }
    }

    fn write(&mut self, data: &[u8]) -> Result<(), Error> {

        match self {
            ConnectionStream::Tcp(stream) => stream.write_all(&data),
            ConnectionStream::Tls(stream) => stream.write_all(&data)
        };
        Ok(())
    }

    fn get_reader(&self) -> BufReader<Box<dyn Read>> {

        if let ConnectionStream::Tcp(stream) = self {
            BufReader::new(Box::new(stream))
        } else if let ConnectionStream::Tls(stream) = self {
            BufReader::new(Box::new(stream))
        } else {
            panic!("No stream");
        }
    }
}

===========================================

// Tried trait object as shown below, but still sale issues.
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


    /// connect() function within the HttpClient impl
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






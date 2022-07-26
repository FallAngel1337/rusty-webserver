use tokio::net::{TcpStream, TcpListener, ToSocketAddrs};
use tokio::io::{AsyncReadExt, AsyncWriteExt, Result as IoResult};
use std::io::ErrorKind;
use std::path::Path;
use httparse::Request;

#[derive(Debug, Clone)]
pub struct HTTPServer<S, P>
where
    S: ToSocketAddrs,
    P: AsRef<Path> + Clone + Send + 'static
{
    addr: S,
    html: P
}

impl<S, P>  HTTPServer<S, P>
where
    S: ToSocketAddrs,
    P: AsRef<Path> + Clone + Send + 'static
{
    pub fn new(addr: S, html: P) -> Self {
        Self { addr, html }
    }

    pub async fn listen(self) -> IoResult<()> {
        let listener = TcpListener::bind(&self.addr).await?;
        
        loop {
            let (socket, addr) = listener.accept().await?;
            println!("Connection from {addr:?}");
            let html = self.html.clone();

            tokio::spawn(async move { Self::default_handler(html, socket).await });
        }
    }        
    
    async fn default_handler(dir: P, mut stream: TcpStream) -> IoResult<()> {
        let mut headers = vec![httparse::EMPTY_HEADER; 64];
        let mut buf = vec![0u8; 1024];
        let mut request = Request::new(&mut headers);
        
        if stream.read(&mut buf).await? == 0 { println!("Connection lost with {:?}", stream.peer_addr()); panic!("a") };
        
        request.parse(&buf).unwrap();
        
        
        let dir = dir.as_ref().display();
        let path = request.path.unwrap();
        let path = if path == "/" { "/index.html" } else { path };
        let response = match std::fs::read_to_string(&format!("{}{}", dir, path)) {
            Ok(html) => format!("HTTP/1.1 200 OK\r\nServer: rusty-server\r\nContent-Length: {}\r\nContent-Type: text/html\r\n\r\n{}", html.len(), html),
            Err(e) => match e.kind() {
                ErrorKind::NotFound => {
                    let html = std::fs::read_to_string(&format!("{}/404.html", dir))?;
                    format!("HTTP/1.1 404 Not Found\r\nServer: rusty-server\r\nContent-Length: {}\r\nContent-Type: text/html\r\n\r\n{}", html.len(), html)
                },
                _ => panic!("Unexpected error...")
            }
        };
        stream.write_all(response.as_bytes()).await?;

        Ok(())
    }
}

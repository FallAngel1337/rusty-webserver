use tokio::net::{TcpStream, TcpListener, ToSocketAddrs};
use tokio::io::{AsyncReadExt, AsyncWriteExt, Result as IoResult};
use std::io::{ErrorKind, Write};
use std::path::Path;
use httparse::Request;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VirtualHost {
    hostname: String,
    dir: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VirtualHosts(Vec<VirtualHost>);

#[derive(Debug, Clone)]
pub struct HTTPServer<S: ToSocketAddrs>
{
    addr: S,
    vhosts: VirtualHosts,
}

impl<S: ToSocketAddrs> HTTPServer<S> {
    pub fn new(config: impl AsRef<Path>, addr: S) -> IoResult<Self> {
        Ok(Self { addr, vhosts: serde_yaml::from_str(&std::fs::read_to_string(config)?).unwrap() })
    }

    pub async fn listen(self) -> IoResult<()> {
        let listener = TcpListener::bind(&self.addr).await?;
        
        loop {
            let (socket, addr) = listener.accept().await?;
            println!("Connection from {addr:?}");
            let vhosts = self.vhosts.clone();
            tokio::spawn(async move { Self::default_handler(vhosts, socket).await });
        }
    }        
    
    async fn default_handler(vhosts: VirtualHosts, mut stream: TcpStream) -> IoResult<()> {
        let mut headers = vec![httparse::EMPTY_HEADER; 64];
        let mut buf = vec![0u8; 1024];
        let mut request = Request::new(&mut headers);
        
        if stream.read(&mut buf).await? == 0 { println!("Connection lost with {:?}", stream.peer_addr()); panic!("a") };
        
        request.parse(&buf).unwrap();
        
        if let Some(host) = request.headers.iter_mut().find(|x| x.name == "Host") {
            let host = std::str::from_utf8(host.value).unwrap();
            if let Some(vhost) = vhosts.iter().find(|f| f.hostname == host) {
                let dir = &vhost.dir;
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
            }
        }

        Ok(())
    }
}


impl VirtualHosts {
    #[inline]
    fn iter(&self) -> impl Iterator<Item = &VirtualHost> {
        self.0.iter()
    }
    
    fn save(&self, config: impl AsRef<Path>) -> IoResult<()> {
        std::fs::File::create(config)?.write_all(serde_yaml::to_string(self).unwrap().as_bytes())?;
        Ok(())
    }
}

impl<const N: usize> From<[VirtualHost; N]> for VirtualHosts {
    fn from(slice: [VirtualHost; N]) -> Self {
        Self(slice.to_vec())
    }
}

impl VirtualHost {
    fn new(hostname: &str, dir: &str) -> Self {
        Self { hostname: hostname.to_owned(), dir: dir.to_owned() }
    }
}
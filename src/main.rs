mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    server::HTTPServer::new("127.0.0.1:5000", "./html").listen().await?;
    Ok(())
}

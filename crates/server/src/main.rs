use server::{tokio::TokioServer, Server};

mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = TokioServer::run("0.0.0.0:8080".into()).await?;
    Ok(())
}

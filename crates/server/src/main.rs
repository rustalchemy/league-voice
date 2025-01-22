use server::{tokio::TokioServer, Server};

mod error;
mod server;

#[tokio::main]
#[cfg(not(tarpaulin_include))]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = TokioServer::run("0.0.0.0:8080".into()).await?;
    Ok(())
}

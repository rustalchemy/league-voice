use client::{tokio::TokioClient, Client};

mod client;
mod error;

#[tokio::main]
#[cfg(not(tarpaulin_include))]
async fn main() {
    let _ = match TokioClient::connect("0.0.0.0:8080".into()).await {
        Ok(client) => client,
        Err(e) => panic!("Failed to connect to server: {}", e),
    };

    println!("Connected to server");
}

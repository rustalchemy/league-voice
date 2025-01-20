use client::{tokio::TokioClient, Client};

mod client;

#[tokio::main]
async fn main() {
    let _ = match TokioClient::connect("0.0.0.0:8080".into()).await {
        Ok(client) => client,
        Err(e) => panic!("Failed to connect to server: {}", e),
    };

    println!("Connected to server");
}

use client::{tokio::TokioClient, Client};

mod client;

#[tokio::main]
async fn main() {
    let mut client = TokioClient::new();
    client.connect("0.0.0.0:8080".into()).unwrap();
}

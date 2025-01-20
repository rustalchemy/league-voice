use client::Client;

mod client;
mod tokio;

fn main() {
    let mut client = tokio::TokioClient::new();
    client.connect("0.0.0.0:8080".into()).unwrap();
}

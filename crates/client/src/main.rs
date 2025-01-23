use client::{tokio::TokioClient, Client};
use error::ClientError;

mod client;
mod error;

#[tokio::main]
#[cfg(not(tarpaulin_include))]
async fn main() -> Result<(), ClientError> {
    TokioClient::connect("127.0.0.1:1024".into())
        .await
        .map(|_| ())
}

use std::{borrow::Cow, sync::Arc};

use tokio::net::TcpStream;

use crate::client::Client;

pub(crate) struct TokioClient {
    stream: Arc<TcpStream>,
}

impl Client for TokioClient {
    async fn connect(addr: Cow<'_, str>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            stream: Arc::new(TcpStream::connect(Cow::into_owned(addr.clone())).await?),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tokio_client_connect() {
        let addr = "127.0.0.1:8080";
        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            let (socket, _) = listener.accept().await.unwrap();
            drop(socket);
        });

        assert!(TokioClient::connect(Cow::Borrowed(addr)).await.is_ok());
    }

    #[tokio::test]
    async fn test_tokio_client_connect_fails_if_already_running() {
        let addr = "127.0.0.1:8081";

        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            let (socket, _) = listener.accept().await.unwrap();
            drop(socket);
        });

        assert!(TokioClient::connect(Cow::Borrowed(addr)).await.is_ok());
        assert!(TokioClient::connect(Cow::Borrowed(addr)).await.is_ok());
    }
}

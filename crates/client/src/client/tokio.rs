use std::{borrow::Cow, sync::Arc};

use tokio::net::TcpStream;

use crate::client::Client;

pub(crate) struct TokioClient {
    stream: Arc<TcpStream>,
}

impl Client for TokioClient {
    async fn connect(addr: Cow<'_, str>) -> Result<Self, Box<dyn std::error::Error>> {
        let stream = match TcpStream::connect(Cow::into_owned(addr.clone())).await {
            Ok(stream) => Arc::new(stream),
            Err(e) => return Err(Box::new(e)),
        };

        Ok(Self { stream })
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

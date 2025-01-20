use std::{borrow::Cow, sync::Arc};

use tokio::net::TcpStream;

use crate::client::Client;

pub(crate) struct TokioClient {
    runtime: tokio::runtime::Runtime,
    stream: Option<Arc<TcpStream>>,
    is_running: Arc<bool>,
}

impl TokioClient {
    pub fn new() -> Self {
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(e) => panic!("Failed to create runtime: {}", e),
        };

        TokioClient {
            runtime,
            stream: None,
            is_running: Arc::new(false),
        }
    }
}

impl Client for TokioClient {
    fn connect(&mut self, addr: Cow<'_, str>) -> Result<(), Box<dyn std::error::Error>> {
        let is_running = Arc::clone(&self.is_running);
        if *is_running {
            return Err("Client is already running".into());
        }

        let stream = match self
            .runtime
            .block_on(async { TcpStream::connect(Cow::into_owned(addr.clone())).await })
        {
            Ok(stream) => Arc::new(stream),
            Err(e) => panic!("Failed to connect to address: {}", e),
        };

        self.stream = Some(stream);
        *Arc::make_mut(&mut self.is_running) = true;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokio_client_connect() {
        let addr = "127.0.0.1:8080";
        let mut client = TokioClient::new();

        client.runtime.spawn(async move {
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            let (socket, _) = listener.accept().await.unwrap();
            drop(socket);
        });

        assert!(client.connect(Cow::Borrowed(addr)).is_ok());
    }

    #[test]
    fn test_tokio_client_connect_fails_if_already_running() {
        let addr = "127.0.0.1:8081";
        let mut client = TokioClient::new();

        client.runtime.spawn(async move {
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            let (socket, _) = listener.accept().await.unwrap();
            drop(socket);
        });

        assert!(client.connect(Cow::Borrowed(addr)).is_ok());
        assert!(client.connect(Cow::Borrowed(addr)).is_err());
    }
}

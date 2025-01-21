use std::{borrow::Cow, sync::Arc};
use tokio::net::TcpListener;

use crate::error::ServerError;

use super::Server;

#[derive(Debug)]
pub(crate) struct TokioServer {
    listener: Arc<TcpListener>,
}

impl Server for TokioServer {
    async fn run(addr: Cow<'_, str>) -> Result<Self, ServerError> {
        let listener = match TcpListener::bind(Cow::into_owned(addr.clone())).await {
            Ok(listener) => Arc::new(listener),
            Err(e) => return Err(ServerError::FailedToBind(e)),
        };
        let cloned_listener = listener.clone();

        tokio::spawn(async move {
            let listener = cloned_listener.clone();
            loop {
                let (_, _) = match listener.accept().await {
                    Ok((socket, _)) => (socket, None::<()>),
                    Err(e) => {
                        println!("Failed to accept connection: {}", e);
                        continue;
                    }
                };
            }
        })
        .await?;

        Ok(Self { listener })
    }
}

#[cfg(test)]
mod tests {
    use tokio::select;

    use super::*;
    use std::{io::Write, net::TcpStream};

    #[tokio::test]
    async fn should_open_a_server_connection_on_given_address() {
        let addr = "127.0.0.1:81";

        let server = tokio::spawn(async move {
            let server = TokioServer::run(Cow::Borrowed(addr)).await;
            assert!(server.is_ok(), "expected server to be running");
            let _ = server.unwrap();
        });

        let client = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_micros(50)).await;
            let mut client = TcpStream::connect(addr).unwrap();
            client.write_all(b"hello")
        });

        select! {
            _ = server => (),
            result = client => {
                assert!(result.is_ok(), "expected client to connect");
            }
        }
    }

    #[tokio::test]
    async fn should_return_error_on_invalid_address() {
        let addr = "127.0.0.1:82";
        let server = tokio::spawn(async move { TokioServer::run(Cow::Borrowed(addr)).await });
        let sec_server = tokio::spawn(async move { TokioServer::run(Cow::Borrowed(addr)).await });

        select! {
            Ok(result) = server => {
                assert!(result.is_ok(), "expected server to start");
            },
            Ok(sec_result) = sec_server => {
                assert!(sec_result.is_err(), "expected server to error with address already in use");
            }
        }
    }
}

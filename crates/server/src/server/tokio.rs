use std::{borrow::Cow, sync::Arc};
use tokio::net::TcpListener;

use super::Server;

pub(crate) struct TokioServer {
    listener: Arc<TcpListener>,
}

impl Server for TokioServer {
    async fn run(addr: Cow<'_, str>) -> Result<Self, Box<dyn std::error::Error>> {
        let listener = match TcpListener::bind(Cow::into_owned(addr.clone())).await {
            Ok(listener) => Arc::new(listener),
            Err(e) => return Err(Box::new(e)),
        };

        let cloned_listener = listener.clone();
        tokio::spawn(async move {
            let listener = listener.clone();
            loop {
                let (_, _) = match listener.accept().await {
                    Ok((socket, _)) => (socket, None::<()>),
                    Err(e) => {
                        println!("Failed to accept connection: {}", e);
                        continue;
                    }
                };
            }
        });

        Ok(Self {
            listener: cloned_listener,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, net::TcpStream};

    #[tokio::test]
    async fn should_open_a_server_connection_on_given_address() {
        let addr = "127.0.0.1:82";
        let server = TokioServer::run(Cow::Borrowed(addr)).await;
        assert!(server.is_ok(), "expected server to be running");
        let _ = server.unwrap();

        let mut client = TcpStream::connect(addr).unwrap();
        assert!(
            client.write_all(b"hello").is_ok(),
            "failed to write to server"
        );
    }

    #[tokio::test]
    async fn should_stop_server_if_it_leaves_scope() {
        let addr = "0.0.0.0:89";
        {
            let server = TokioServer::run(Cow::Borrowed(addr)).await;
            assert!(server.is_ok(), "expected server to be running");
            let _ = server.unwrap();
        }

        let client = TcpStream::connect(addr);
        assert!(client.is_err(), "expected connection to fail");
    }

    #[tokio::test]
    async fn should_return_error_on_invalid_address() {
        let addr = "0.0.0.0:83";
        let _ = TokioServer::run(Cow::Borrowed(addr)).await.unwrap();

        let server_err = TokioServer::run(Cow::Borrowed(addr)).await;
        assert!(server_err.is_err(), "expected an error");
    }
}

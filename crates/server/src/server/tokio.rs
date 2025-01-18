use std::borrow::Cow;
use tokio::net::TcpListener;

use super::Server;

struct TokioServer {
    listener: TcpListener,
}

impl TokioServer {
    pub async fn new(addr: Cow<'_, str>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(TokioServer {
            listener: TcpListener::bind(Cow::into_owned(addr)).await?,
        })
    }
}

impl Server for TokioServer {
    async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let (stream, _) = self.listener.accept().await?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, net::TcpStream};

    #[tokio::test]
    async fn should_open_a_server_connection_on_given_address() {
        let addr = "127.0.0.1:8080";
        let _server = TokioServer::new(Cow::Borrowed(addr)).await.unwrap();

        let mut client = TcpStream::connect(addr).unwrap();
        assert!(
            client.write_all(b"hello").is_ok(),
            "failed to write to server"
        );
    }
}

use common::packet::{packet_type::PacketType, ConnectPacket, Packet};
use std::{borrow::Cow, sync::Arc};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{client::Client, error::ClientError};

pub(crate) struct TokioClient {
    stream: Arc<TcpStream>,
}

impl Client for TokioClient {
    async fn connect(addr: Cow<'_, str>) -> Result<Self, ClientError> {
        let mut stream = TcpStream::connect(Cow::into_owned(addr.clone())).await?;
        stream.set_nodelay(true)?;

        // let packet: Vec<u8> = Packet::new(ConnectPacket)?.into();
        // stream.write_all(&packet).await?;
        // stream.flush().await?;

        Ok(Self {
            stream: Arc::new(stream),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::select;

    #[tokio::test]
    async fn test_tokio_client_connect() {
        let addr = "127.0.0.1:8111";

        let server = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            let (socket, _) = listener.accept().await.unwrap();
            drop(socket);

            Ok::<(), std::io::Error>(())
        });
        let client = tokio::spawn(async move { TokioClient::connect(Cow::Borrowed(addr)).await });
        select! {
            Ok(result) = server => {
                assert!(result.is_ok(), "expected server to start");
            },
            Ok(result) = client => {
                assert!(result.is_ok(), "expected client to connect");
            }
        }
    }
}

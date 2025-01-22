use super::Server;
use crate::error::ServerError;
use common::packet::{packet_type::PacketType, Packet};
use std::borrow::Cow;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

const MAX_PACKET_SIZE: usize = 512;

#[derive(Debug)]
pub(crate) struct TokioServer {}

impl TokioServer {
    async fn handle_stream(stream: &mut TcpStream) -> Result<(), ServerError> {
        let mut buffer = Vec::with_capacity(MAX_PACKET_SIZE * 2);
        loop {
            let mut temp_buffer = [0; MAX_PACKET_SIZE];
            let bytes_read = stream.read(&mut temp_buffer).await?;
            if bytes_read == 0 {
                return Err(ServerError::ConnectionClosedByPeer);
            }

            buffer.extend_from_slice(&temp_buffer[..bytes_read]);

            while let Ok(packet) = Packet::decode(&mut buffer) {
                TokioServer::process_packet(packet).await?;
            }

            if buffer.len() > MAX_PACKET_SIZE * 2 {
                return Err(ServerError::FailedToProcessPacket);
            }
        }
    }
}

impl Server for TokioServer {
    async fn run(addr: Cow<'_, str>) -> Result<(), ServerError> {
        let listener = TcpListener::bind(Cow::into_owned(addr.clone())).await?;

        loop {
            let (mut stream, _) = listener.accept().await.unwrap();

            tokio::spawn(async move {
                if let Err(e) = TokioServer::handle_stream(&mut stream).await {
                    println!("Error: {}", e);
                }
                stream.shutdown().await
            });
        }
    }

    async fn process_packet(packet: Packet) -> Result<(), ServerError> {
        println!(
            "Processing packet: {:?}",
            PacketType::deserialize(packet.data.as_slice())?
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Error;
    use std::time::Duration;
    use tokio::net::TcpStream;
    use tokio::time::sleep;
    use tokio::{io::AsyncWriteExt, select};

    async fn check_for_closed(mut client: TcpStream) -> Result<(), std::io::Error> {
        let mut buffer = [1; MAX_PACKET_SIZE];
        let n = client.read(&mut buffer[..]).await?;

        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "connection closed",
            ));
        }

        Ok(())
    }

    #[tokio::test]
    async fn should_open_a_server_connection_on_given_address() {
        let addr = "127.0.0.1:1025";

        let server = tokio::spawn(async move { TokioServer::run(Cow::Borrowed(addr)).await });
        let client = tokio::spawn(async move {
            let mut client = TcpStream::connect(addr).await?;
            client
                .write_all(
                    Packet::new(PacketType::Connect)
                        .unwrap()
                        .encode()
                        .as_slice(),
                )
                .await?;
            client.flush().await
        });

        select! {
            Ok(result) = server => {
                assert!(result.is_ok(), "expected server to start");
            },
            Ok(result) = client => {
                assert!(result.is_ok(), "expected client to connect");
            }
        }
    }

    #[tokio::test]
    async fn should_return_error_on_invalid_address() {
        let addr = "127.0.0.1:1026";
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

    #[tokio::test]
    async fn should_process_multiple_packets() {
        let addr = "127.0.0.1:1027";

        let server = tokio::spawn(async move { TokioServer::run(Cow::Borrowed(addr)).await });
        let client = tokio::spawn(async move {
            let packet = Packet::new(PacketType::Audio(Vec::new())).unwrap().encode();
            let connect = Packet::new(PacketType::Connect).unwrap().encode();
            let disconnect = Packet::new(PacketType::Disconnect).unwrap().encode();

            let mut client = TcpStream::connect(addr).await?;
            client.write_all(connect.as_slice()).await?;
            client.flush().await?;

            client.write_all(packet.as_slice()).await?;
            client.write_all(packet.as_slice()).await?;
            client.write_all(disconnect.as_slice()).await?;
            client.flush().await?;

            sleep(Duration::from_secs(1)).await;

            Ok::<(), Error>(())
        });

        select! {
            Ok(result) = server => {
                assert!(result.is_ok(), "expected server to keep running");
            },
            Ok(result) = client => {
                assert!(result.is_ok(), "expected client to connect");
            }
        }
    }

    #[tokio::test]
    async fn should_close_connection_on_invalid_packet() {
        let addr = "127.0.0.1:1028";

        let server = tokio::spawn(async move { TokioServer::run(Cow::Borrowed(addr)).await });
        let client = tokio::spawn(async move {
            let mut client = TcpStream::connect(addr).await?;
            client.write_all(&[0, 0, 0, 0]).await?;
            check_for_closed(client).await
        });

        select! {
            Ok(result) = server => {
                assert!(result.is_ok(), "expected server to keep running");
            },
            Ok(result) = client => {
                assert!(result.is_err(), "expected client to error with invalid packet");
            }
        }
    }

    #[tokio::test]
    async fn should_close_connection_on_empty_packet() {
        let addr = "127.0.0.1:1029";

        let server = tokio::spawn(async move { TokioServer::run(Cow::Borrowed(addr)).await });
        let client = tokio::spawn(async move {
            let mut client = TcpStream::connect(addr).await?;
            client.shutdown().await?;

            check_for_closed(client).await
        });

        select! {
            Ok(result) = server => {
                assert!(result.is_ok(), "expected server to keep running");
            },
            Ok(result) = client => {
                assert!(result.is_err(), "expected client to error with invalid packet");
            }
        }
    }

    #[tokio::test]
    async fn should_close_connection_on_buffer_overflow() {
        let addr = "127.0.0.1:1030";

        let server = tokio::spawn(async move { TokioServer::run(Cow::Borrowed(addr)).await });
        let client = tokio::spawn(async move {
            let mut client = TcpStream::connect(addr).await?;
            let packet = Packet::new(PacketType::Audio(Vec::new())).unwrap().encode();
            let mut buffer = vec![1; 1024 * 3];
            buffer.extend_from_slice(&packet);

            client.write_all(&buffer).await?;
            client.flush().await?;

            check_for_closed(client).await
        });

        select! {
            Ok(result) = server => {
                assert!(result.is_ok(), "expected server to keep running");
            },
            Ok(result) = client => {
                assert!(result.is_err(), "expected client to error with buffer overflow");
            }
        }
    }
}

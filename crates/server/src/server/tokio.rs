use super::Server;
use crate::error::ServerError;
use bincode::ErrorKind;
use common::packet::Packet;
use std::borrow::Cow;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

const MAX_PACKET_SIZE: usize = 1024;

#[derive(Debug)]
pub(crate) struct TokioServer {}

impl TokioServer {
    async fn handle_stream(stream: &mut TcpStream) -> Result<(), ServerError> {
        let mut buffer = Vec::with_capacity(MAX_PACKET_SIZE * 2);
        loop {
            let mut temp_buffer = [0; 1024];
            let bytes_read = stream
                .read(&mut temp_buffer)
                .await
                .map_err(|e| ServerError::IoError(e))?;

            if bytes_read == 0 {
                return Err(ServerError::ConnectionClosedByPeer);
            }

            buffer.extend_from_slice(&temp_buffer[..bytes_read]);

            match Packet::decode(&mut buffer)
                .map_err(|e| ServerError::FailedToDecodePacket(ErrorKind::Custom(e)))
            {
                Ok(packet) => {
                    let _ = Self::process_packet(packet).await?;
                }
                Err(e) => {
                    return Err(ServerError::FailedToProcessPacket(e.to_string()));
                }
            }

            if buffer.len() > MAX_PACKET_SIZE * 2 {
                return Err(ServerError::FailedToProcessPacket(
                    "Buffer overflow: possible protocol error".to_string(),
                ));
            }
        }
    }

    async fn handle_error(stream: &mut TcpStream, error: ServerError) {
        println!("Error: {}", error);
        match stream.shutdown().await {
            Err(e) => {
                println!("Failed to shutdown stream: {}", e);
                return;
            }
            Ok(_) => println!("Stream shutdown successfully"),
        }
    }
}

impl Server for TokioServer {
    async fn run(addr: Cow<'_, str>) -> Result<Self, ServerError> {
        let listener = TcpListener::bind(Cow::into_owned(addr.clone()))
            .await
            .map_err(|e| ServerError::FailedToBind(e))?;

        tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                tokio::spawn(async move {
                    match TokioServer::handle_stream(&mut stream).await {
                        Err(e) => TokioServer::handle_error(&mut stream, e).await,
                        Ok(_) => (),
                    }
                });
            }
        })
        .await?;

        Ok(Self {})
    }

    async fn process_packet(packet: Packet) -> Result<(), ServerError> {
        println!("Received packet: {:?}", packet);
        let packet_type = bincode::deserialize(&packet.data)
            .map_err(|e: Box<bincode::ErrorKind>| ServerError::FailedToDecodePacketType(*e))?;

        match packet_type {
            common::packet::PacketType::Connect => {
                println!("Received connect packet");
            }
            common::packet::PacketType::Disconnect => {
                println!("Received disconnect packet");
            }
            common::packet::PacketType::Audio(data) => {
                println!("Received audio packet: {:?}", data);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Error;
    use std::time::Duration;

    use super::*;
    use common::packet::PacketType;
    use tokio::net::TcpStream;
    use tokio::time::sleep;
    use tokio::{io::AsyncWriteExt, select};

    async fn check_for_closed(mut client: TcpStream) -> Result<(), std::io::Error> {
        let mut buffer = [1; 1024];
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
        let addr = "127.0.0.1:81";

        let server = tokio::spawn(async move { TokioServer::run(Cow::Borrowed(addr)).await });
        let client = tokio::spawn(async move {
            let mut client = TcpStream::connect(addr).await?;
            client
                .write_all(
                    Packet::new(PacketType::Connect)
                        .encode()
                        .unwrap()
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

    #[tokio::test]
    async fn should_process_multiple_packets() {
        let addr = "127.0.0.1:83";

        let server = tokio::spawn(async move { TokioServer::run(Cow::Borrowed(addr)).await });
        let client = tokio::spawn(async move {
            let packet = Packet::new(PacketType::Audio(Vec::new())).encode().unwrap();
            let connect = Packet::new(PacketType::Connect).encode().unwrap();
            let disconnect = Packet::new(PacketType::Disconnect).encode().unwrap();

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
                assert!(result.is_err(), "expected server to keep running");
            },
            Ok(result) = client => {
                assert!(result.is_ok(), "expected client to connect");
            }
        }
    }

    #[tokio::test]
    async fn should_close_connection_on_invalid_packet() {
        let addr = "127.0.0.1:84";

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
        let addr = "127.0.0.1:85";

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
        let addr = "127.0.0.1:86";

        let server = tokio::spawn(async move { TokioServer::run(Cow::Borrowed(addr)).await });
        let client = tokio::spawn(async move {
            let mut client = TcpStream::connect(addr).await?;
            let packet = Packet::new(PacketType::Audio(Vec::new())).encode().unwrap();
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

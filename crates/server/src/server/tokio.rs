use super::Server;
use crate::{error::ServerError, packets::PacketHandler};
use common::packet::{ids::PacketId, Packet};
use std::{borrow::Cow, collections::HashMap, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

const MAX_PACKET_SIZE: usize = 512;

type PacketHandlerMap = HashMap<u8, Box<dyn PacketHandler>>;

pub(crate) struct TokioServer {
    handlers: Arc<PacketHandlerMap>,
}

impl TokioServer {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(PacketHandlerMap::new()),
        }
    }

    async fn handle_stream(
        handlers: Arc<PacketHandlerMap>,
        stream: &mut TcpStream,
    ) -> Result<(), ServerError> {
        let mut buffer = Vec::with_capacity(MAX_PACKET_SIZE * 2);
        loop {
            let handlers = handlers.clone();
            let mut temp_buffer = [0; MAX_PACKET_SIZE];
            let bytes_read = stream.read(&mut temp_buffer).await?;
            if bytes_read == 0 {
                return Err(ServerError::ConnectionClosedByPeer);
            }

            buffer.extend_from_slice(&temp_buffer[..bytes_read]);

            while let Ok(packet) = Packet::decode(&mut buffer) {
                TokioServer::process_packet(handlers.clone(), packet).await?;
            }

            if buffer.len() > MAX_PACKET_SIZE * 2 {
                println!("Buffer length: {}", buffer.len());
                return Err(ServerError::FailedToProcessPacket);
            }
        }
    }

    pub fn add_handler(&mut self, id: PacketId, handler: Box<dyn PacketHandler>) {
        Arc::get_mut(&mut self.handlers)
            .unwrap()
            .insert(id as u8, handler);
    }
}

impl Server for TokioServer {
    type Handlers = PacketHandlerMap;

    async fn run(&mut self, addr: Cow<'_, str>) -> Result<(), ServerError> {
        let listener = TcpListener::bind(Cow::into_owned(addr.clone())).await?;

        let handlers = self.handlers.clone();
        loop {
            let handlers = handlers.clone();
            let (mut stream, _) = listener.accept().await.unwrap();

            tokio::spawn(async move {
                if let Err(e) = TokioServer::handle_stream(handlers.clone(), &mut stream).await {
                    println!("Error: {}", e);
                }
                stream.shutdown().await
            });
        }
    }

    async fn process_packet(
        handlers: Arc<Self::Handlers>,
        packet: Packet,
    ) -> Result<(), ServerError> {
        let packet_id = match PacketId::from_u8(packet.packet_id) {
            Some(packet_id) => packet_id,
            None => return Err(ServerError::InvalidPacket),
        };

        let handler = match handlers.get(&(packet_id.clone() as u8)) {
            Some(handler) => handler.as_ref(),
            None => {
                return Err(ServerError::HandlerNotFound);
            }
        };

        PacketHandler::process(handler, &packet_id.clone(), packet.data.as_slice()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packets::handlers;
    use common::packet::{AudioPacket, ConnectPacket, DisconnectPacket};
    use std::io::Error;
    use std::time::Duration;
    use tokio::net::TcpStream;
    use tokio::time::sleep;
    use tokio::{io::AsyncWriteExt, select};

    async fn start_server(addr: &str) -> Result<(), ServerError> {
        let mut server = TokioServer::new();
        server.add_handler(
            PacketId::ConnectPacket,
            Box::new(handlers::connect::ConnectHandler {}),
        );
        server.add_handler(
            PacketId::AudioPacket,
            Box::new(handlers::audio::AudioHandler {}),
        );
        server.add_handler(
            PacketId::DisconnectPacket,
            Box::new(handlers::disconnect::DisconnectHandler {}),
        );
        server.run(Cow::Borrowed(addr)).await
    }

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

        let server = tokio::spawn(async move { start_server(addr).await });
        let client = tokio::spawn(async move {
            let mut client = TcpStream::connect(addr).await?;
            client
                .write_all(Packet::new(ConnectPacket).unwrap().encode().as_slice())
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
        let server = tokio::spawn(async move { start_server(addr).await });
        let sec_server = tokio::spawn(async move { start_server(addr).await });

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

        let server = tokio::spawn(async move { start_server(addr).await });
        let client = tokio::spawn(async move {
            let packet = Packet::new(AudioPacket { track: vec![1] })
                .unwrap()
                .encode();
            let connect = Packet::new(ConnectPacket).unwrap().encode();
            let disconnect = Packet::new(DisconnectPacket).unwrap().encode();

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

        let server = tokio::spawn(async move { start_server(addr).await });
        let client = tokio::spawn(async move {
            let mut client = TcpStream::connect(addr).await?;
            client.write_all(&[0, 0, 0, 0, 18]).await?;
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

        let server = tokio::spawn(async move { start_server(addr).await });
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

        let server = tokio::spawn(async move { start_server(addr).await });
        let client = tokio::spawn(async move {
            let mut client = TcpStream::connect(addr).await?;
            let packet = Packet::new(AudioPacket { track: Vec::new() })
                .unwrap()
                .encode();
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

    #[tokio::test]
    async fn should_close_connection_on_handler_not_found() {
        let addr = "127.0.0.1:1031";

        let server = tokio::spawn(async move {
            let mut server = TokioServer::new();
            server.run(Cow::Borrowed(addr)).await
        });
        let client = tokio::spawn(async move {
            let connect = Packet::new(ConnectPacket).unwrap().encode();

            let mut client = TcpStream::connect(addr).await?;
            client.write_all(connect.as_slice()).await?;
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

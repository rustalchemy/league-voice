use super::{Clients, Server};
use crate::{
    error::ServerError,
    packets::{PacketData, PacketHandler},
    server::client::Client,
};
use common::packet::{ids::PacketId, Packet, MAX_PACKET_SIZE};
use std::{borrow::Cow, collections::HashMap, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    select,
    sync::Mutex,
};
use uuid::Uuid;

type PacketHandlerMap = HashMap<u8, Box<dyn PacketHandler>>;

pub(crate) struct TokioServer {
    handlers: Arc<PacketHandlerMap>,
    clients: Arc<Clients>,
}

impl TokioServer {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(PacketHandlerMap::new()),
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn handle_stream(
        client_id: Uuid,
        handlers: Arc<PacketHandlerMap>,
        clients: Arc<Clients>,
        stream: TcpStream,
    ) -> Result<(), ServerError> {
        let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(32);
        let (mut read, mut write) = stream.into_split();

        let read_handle = tokio::spawn(async move {
            let mut buffer = Vec::with_capacity(MAX_PACKET_SIZE * 2);
            loop {
                let handlers = handlers.clone();
                let mut temp_buffer = [0; MAX_PACKET_SIZE];
                let bytes_read = read.read(&mut temp_buffer).await?;
                if bytes_read == 0 {
                    return Err(ServerError::ConnectionClosedByPeer);
                }

                buffer.extend_from_slice(&temp_buffer[..bytes_read]);

                while let Ok(packet) = Packet::decode(&mut buffer) {
                    if let Err(e) = Self::process_packet(client_id, handlers.clone(), packet).await
                    {
                        println!("Processing packet error: {}", e);
                        return Err(e);
                    }
                }

                if buffer.len() > MAX_PACKET_SIZE * 2 {
                    println!("Buffer length: {}", buffer.len());
                    return Err(ServerError::FailedToProcessPacket);
                }
            }
        });

        let write_handle = tokio::spawn(async move {
            while let Some(packet) = write_rx.recv().await {
                write.write_all(&packet).await?;
                write.flush().await?;
            }

            Ok(())
        });

        {
            let mut clients = clients.lock().await;
            clients.insert(client_id, Client::new(client_id, write_tx));
        }

        select! {
            Ok(read_result) = read_handle => {
                read_result
            },
            Ok(write_result) = write_handle => {
                write_result
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
        println!("Server started on: {}", addr);

        let handlers = self.handlers.clone();
        let clients = self.clients.clone();
        loop {
            let handlers = handlers.clone();
            let clients = clients.clone();

            let (stream, _) = listener.accept().await.unwrap();

            tokio::spawn(async move {
                let client_id = Uuid::new_v4();
                println!("Client connected: {}", client_id);

                if let Err(e) =
                    TokioServer::handle_stream(client_id, handlers.clone(), clients.clone(), stream)
                        .await
                {
                    println!("Error: {}", e);
                }

                {
                    let mut clients = clients.lock().await;
                    clients.remove(&client_id);
                    println!("Client disconnected: {}", client_id);
                }
            });
        }
    }

    async fn process_packet(
        client_id: Uuid,
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

        handler
            .process(PacketData::new(client_id, packet_id, packet.data))
            .await
    }

    fn clients(&self) -> Arc<Clients> {
        self.clients.clone()
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
            Box::new(handlers::audio::AudioHandler(server.clients().clone())),
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
                .write_all(Packet::new(ConnectPacket).encode().as_slice())
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

    async fn connect_and_send_packets(addr: &str) -> Result<(), Error> {
        let packet = Packet::new(AudioPacket { track: vec![1] }).encode();
        let connect = Packet::new(ConnectPacket).encode();
        let disconnect = Packet::new(DisconnectPacket).encode();

        let mut client = TcpStream::connect(addr).await?;
        client.write_all(connect.as_slice()).await?;
        client.flush().await?;

        sleep(Duration::from_millis(2)).await;

        client.write_all(packet.as_slice()).await?;
        client.write_all(packet.as_slice()).await?;
        client.write_all(disconnect.as_slice()).await?;
        client.flush().await?;

        sleep(Duration::from_millis(20)).await;

        Ok::<(), Error>(())
    }

    #[tokio::test]
    async fn should_process_multiple_packets() {
        let addr = "127.0.0.1:1027";

        let server = tokio::spawn(async move { start_server(addr).await });
        let client = tokio::spawn(async move { connect_and_send_packets(addr).await });

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
    async fn should_process_multiple_clients() {
        let addr = "127.0.0.1:1032";
        let server = tokio::spawn(async move { start_server(addr).await });
        let client = tokio::spawn(async move { connect_and_send_packets(addr).await });
        let second_client = tokio::spawn(async move { connect_and_send_packets(addr).await });

        select! {
            Ok(result) = server => {
                assert!(result.is_ok(), "expected server to keep running");
            },
            Ok(result) = client => {
                assert!(result.is_ok(), "expected client to connect");
            }
            Ok(result) = second_client => {
                assert!(result.is_ok(), "expected second_client to connect");
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
            let packet = Packet::new(AudioPacket { track: Vec::new() }).encode();
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
            let connect = Packet::new(ConnectPacket).encode();

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

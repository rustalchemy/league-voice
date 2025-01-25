use std::sync::Arc;

use crate::{
    error::ServerError,
    packets::{PacketData, PacketHandler},
    server::client::Clients,
};
use common::packet::{ids::PacketId, packet_type::PacketType, AudioPacket};

pub struct AudioHandler(pub Arc<Clients>);

#[async_trait::async_trait]
impl PacketHandler for AudioHandler {
    async fn process(&self, data: PacketData) -> Result<(), ServerError> {
        if &data.packet_id != &PacketId::AudioPacket {
            return Err(ServerError::InvalidHandlerPacketId);
        }

        let packet: AudioPacket =
            AudioPacket::decode(&data.packet).map_err(|_| ServerError::InvalidPacket)?;
        println!("Processing audio packet: {:?}", packet);

        {
            for client in self.0.lock().await.values() {
                if client.id() != data.client_id {
                    match client.send(&data.packet).await {
                        Ok(_) => {
                            println!("{:?} -> {:?}", data.client_id, client.id());
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    };
                }
            }
        }

        println!("Processed audio packet: {:?}", packet);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::server::client::Client;

    use super::*;
    use ::tokio::sync::{mpsc, Mutex};
    use common::packet::ids::PacketId;
    use std::collections::HashMap;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_audio_handler() {
        let clients = Arc::new(Mutex::new(HashMap::new()));
        let (tx, mut _read_tx) = mpsc::channel(1);
        {
            clients
                .lock()
                .await
                .insert(Uuid::new_v4(), Client::new(Uuid::new_v4(), tx));
        }

        assert!(
            AudioHandler(clients)
                .process(PacketData::new(
                    Default::default(),
                    PacketId::AudioPacket,
                    AudioPacket {
                        track: vec![1, 2, 3, 4, 5]
                    }
                    .encode()
                    .unwrap()
                ))
                .await
                .is_ok(),
            "Expected handler to process packet"
        );
    }

    #[tokio::test]
    async fn test_audio_handler_invalid_packet_id() {
        // assert!(
        //     AudioHandler {}
        //         .process(PacketData::new(
        //             Default::default(),
        //             PacketId::ConnectPacket,
        //             AudioPacket::default().encode().unwrap()
        //         ))
        //         .await
        //         .is_err(),
        //     "Expected handler to return error for invalid packet id"
        // );
    }
}

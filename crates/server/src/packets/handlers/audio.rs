use std::sync::Arc;

use crate::{
    error::ServerError,
    packets::{PacketData, PacketHandler},
    server::client::Clients,
};
use common::packet::{ids::PacketId, packet_type::PacketType, AudioPacket, Packet};

pub struct AudioHandler(pub Arc<Clients>);

#[async_trait::async_trait]
impl PacketHandler for AudioHandler {
    async fn process(&self, data: PacketData) -> Result<(), ServerError> {
        if &data.packet_id != &PacketId::AudioPacket {
            return Err(ServerError::InvalidHandlerPacketId);
        }

        let packet = AudioPacket::decode(&data.data).map_err(|_| ServerError::InvalidPacket)?;
        let packet = Packet::new(packet).map_err(|_| ServerError::InvalidPacket)?;

        let encoded_packet = packet.encode();

        for client in self.0.lock().await.values() {
            if client.id() == data.client_id {
                client.send(&encoded_packet).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::server::client::Client;

    use super::*;
    use ::tokio::sync::{mpsc, Mutex};
    use common::packet::{ids::PacketId, packet_type::PacketType, AudioPacket};
    use std::collections::HashMap;
    use tokio::select;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_audio_handler() {
        let clients = Arc::new(Mutex::new(HashMap::new()));
        let (tx, mut read_tx) = mpsc::channel(1);
        let (tx_2, mut read_tx_2) = mpsc::channel(1);

        let client = Client::new(Uuid::new_v4(), tx);
        let second_client = Client::new(Uuid::new_v4(), tx_2);

        {
            clients.lock().await.insert(client.id(), client);
            clients
                .lock()
                .await
                .insert(second_client.id(), second_client);
        }

        let audio_packet = AudioPacket {
            track: vec![1, 2, 3, 4, 5],
        }
        .encode()
        .unwrap();

        let packet = Packet::new(AudioPacket {
            track: vec![1, 2, 3, 4, 5],
        })
        .unwrap()
        .encode();

        assert!(
            AudioHandler(clients)
                .process(PacketData::new(
                    Default::default(),
                    PacketId::AudioPacket,
                    audio_packet.clone(),
                ))
                .await
                .is_ok(),
            "Expected handler to process packet"
        );

        select! {
            result = tokio::spawn(async move { read_tx.recv().await }) => {
                assert!(result.is_ok());
                assert_eq!(result.unwrap().unwrap(), packet, "Expected packet to be sent to first client");
            }
            result = tokio::spawn(async move { read_tx_2.recv().await }) => {
                assert!(result.is_ok());
                assert_eq!(result.unwrap().unwrap(), packet, "Expected packet to be sent to second client");
            }
        }
    }

    #[tokio::test]
    async fn test_audio_handler_invalid_packet_id() {
        assert!(
            AudioHandler(Arc::new(Mutex::new(HashMap::new())))
                .process(PacketData::new(
                    Default::default(),
                    PacketId::ConnectPacket,
                    AudioPacket::default().encode().unwrap()
                ))
                .await
                .is_err(),
            "Expected handler to return error for invalid packet id"
        );
    }
}

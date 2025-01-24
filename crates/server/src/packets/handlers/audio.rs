use std::sync::Arc;

use crate::{
    error::ServerError,
    packets::{PacketData, PacketHandler},
    server::Clients,
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

        let clients = self.0.lock().await;
        for client in clients.values() {
            if client.id() != data.client_id {
                client.send(&packet.encode().unwrap()).await?;
                println!("Sent audio packet to client: {:?}", client.id());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::packet::ids::PacketId;

    #[tokio::test]
    async fn test_audio_handler() {
        // assert!(
        //     AudioHandler {}
        //         .process(PacketData::new(
        //             Default::default(),
        //             PacketId::AudioPacket,
        //             AudioPacket::default().encode().unwrap()
        //         ))
        //         .await
        //         .is_ok(),
        //     "Expected handler to process packet"
        // );
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

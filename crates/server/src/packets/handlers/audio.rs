use crate::{error::ServerError, packets::PacketHandler};
use common::packet::{ids::PacketId, packet_type::PacketType, AudioPacket};

#[derive(Debug)]
pub struct AudioHandler;

#[async_trait::async_trait]
impl PacketHandler for AudioHandler {
    async fn process(&self, packet_id: &PacketId, packet: &[u8]) -> Result<(), ServerError> {
        if packet_id != &PacketId::AudioPacket {
            return Err(ServerError::InvalidHandlerPacketId);
        }

        let packet = AudioPacket::decode(packet).map_err(|_| ServerError::InvalidPacket)?;
        println!("Processing audio packet: {:?}", packet);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::packet::ids::PacketId;

    #[tokio::test]
    async fn test_audio_handler() {
        let handler = AudioHandler {};
        let packet_bytes = AudioPacket::default().encode().unwrap();
        let packet_id = PacketId::AudioPacket;

        assert!(
            handler.process(&packet_id, &packet_bytes).await.is_ok(),
            "Expected handler to process packet"
        );
    }

    #[tokio::test]
    async fn test_audio_handler_invalid_packet_id() {
        let handler = AudioHandler {};
        let packet_bytes = AudioPacket::default().encode().unwrap();
        let packet_id = PacketId::ConnectPacket;

        assert!(
            handler.process(&packet_id, &packet_bytes).await.is_err(),
            "Expected handler to return error for invalid packet id"
        );
    }
}

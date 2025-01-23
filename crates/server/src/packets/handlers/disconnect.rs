use crate::{error::ServerError, packets::PacketHandler};
use common::packet::{ids::PacketId, packet_type::PacketType, DisconnectPacket};

#[derive(Debug)]
pub struct DisconnectHandler {}

#[async_trait::async_trait]
impl PacketHandler for DisconnectHandler {
    async fn process(&self, packet_id: &PacketId, packet: &[u8]) -> Result<(), ServerError> {
        if packet_id != &PacketId::DisconnectPacket {
            return Err(ServerError::InvalidHandlerPacketId);
        }

        let packet = DisconnectPacket::decode(packet).map_err(|_| ServerError::InvalidPacket)?;
        println!("Processing disconnect packet: {:?}", packet);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::packet::ids::PacketId;

    #[tokio::test]
    async fn test_disconnect_handler() {
        assert!(
            DisconnectHandler {}
                .process(
                    &PacketId::DisconnectPacket,
                    &DisconnectPacket::default().encode().unwrap()
                )
                .await
                .is_ok(),
            "Expected handler to process packet"
        );
    }

    #[tokio::test]
    async fn test_disconnect_handler_invalid_packet_id() {
        assert!(
            DisconnectHandler {}
                .process(
                    &PacketId::AudioPacket,
                    &DisconnectPacket::default().encode().unwrap()
                )
                .await
                .is_err(),
            "Expected handler to return error for invalid packet id"
        );
    }
}

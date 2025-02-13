use crate::{
    error::ServerError,
    packets::{PacketData, PacketHandler},
};
use common::packet::{ids::PacketId, packet_type::PacketType, DisconnectPacket};

#[derive(Debug)]
pub struct DisconnectHandler {}

#[async_trait::async_trait]
impl PacketHandler for DisconnectHandler {
    async fn process(&self, data: PacketData) -> Result<(), ServerError> {
        if &data.packet_id != &PacketId::DisconnectPacket {
            return Err(ServerError::InvalidHandlerPacketId);
        }

        let packet =
            DisconnectPacket::decode(&data.data).map_err(|_| ServerError::InvalidPacket)?;
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
                .process(PacketData::new(
                    Default::default(),
                    PacketId::DisconnectPacket,
                    DisconnectPacket::default().encode()
                ))
                .await
                .is_ok(),
            "Expected handler to process packet"
        );
    }

    #[tokio::test]
    async fn test_disconnect_handler_invalid_packet_id() {
        assert!(
            DisconnectHandler {}
                .process(PacketData::new(
                    Default::default(),
                    PacketId::AudioPacket,
                    DisconnectPacket::default().encode()
                ))
                .await
                .is_err(),
            "Expected handler to return error for invalid packet id"
        );
    }
}

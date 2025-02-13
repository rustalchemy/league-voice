use crate::{
    error::ServerError,
    packets::{PacketData, PacketHandler},
};
use common::packet::{ids::PacketId, packet_type::PacketType, ConnectPacket};

#[derive(Debug, Default)]
pub struct ConnectHandler {}

#[async_trait::async_trait]
impl PacketHandler for ConnectHandler {
    async fn process(&self, data: PacketData) -> Result<(), ServerError> {
        if &data.packet_id != &PacketId::ConnectPacket {
            return Err(ServerError::InvalidHandlerPacketId);
        }

        let packet = ConnectPacket::decode(&data.data).map_err(|_| ServerError::InvalidPacket)?;
        println!("Processing connect packet: {:?}", packet);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::packet::ids::PacketId;

    #[tokio::test]
    async fn test_connect_handler() {
        assert!(
            ConnectHandler {}
                .process(PacketData::new(
                    Default::default(),
                    PacketId::ConnectPacket,
                    ConnectPacket::default().encode()
                ))
                .await
                .is_ok(),
            "Expected handler to process packet"
        );
    }

    #[tokio::test]
    async fn test_connect_handler_invalid_packet_id() {
        assert!(
            ConnectHandler {}
                .process(PacketData::new(
                    Default::default(),
                    PacketId::AudioPacket,
                    ConnectPacket::default().encode()
                ))
                .await
                .is_err(),
            "Expected handler to return error for invalid packet id"
        );
    }
}

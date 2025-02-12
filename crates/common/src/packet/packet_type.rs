use super::ids::PacketId;
use serde::{de::DeserializeOwned, Serialize};

pub trait PacketType: DeserializeOwned + Serialize {
    fn encode(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    fn decode(data: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(data)
    }

    fn packet_id() -> PacketId;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::types::connect::ConnectPacket;

    #[test]
    fn should_encode_and_decode_packet_type() {
        let packet_type = ConnectPacket;
        let deserialized = PacketType::decode(&PacketType::encode(&packet_type)).unwrap();
        assert_eq!(packet_type, deserialized);
    }
}

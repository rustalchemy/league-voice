use super::ids::PacketId;
use serde::{de::DeserializeOwned, Serialize};

pub trait PacketType: DeserializeOwned + Serialize {
    fn encode(&self) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        bincode::serialize(self)
    }

    fn decode(data: &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
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
        let deserialized = PacketType::decode(&PacketType::encode(&packet_type).unwrap()).unwrap();
        assert_eq!(packet_type, deserialized);
    }
}

use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum PacketType {
    Connect,
    Disconnect,
    Audio(Vec<u8>),
}

impl PacketType {
    pub fn serialize(&self) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        bincode::serialize(self)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
        bincode::deserialize(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_serialize_and_deserialize_packet_type() {
        let packet_type = PacketType::Connect;
        let deserialized = PacketType::deserialize(&packet_type.serialize().unwrap()).unwrap();
        assert_eq!(packet_type, deserialized);
    }
}

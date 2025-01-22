pub mod error;
pub mod packet_type;

use error::DecodeError;
use packet_type::PacketType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Packet {
    pub length: u32,
    pub data: Vec<u8>,
}

impl Packet {
    pub fn new(packet_type: PacketType) -> Result<Self, Box<bincode::ErrorKind>> {
        let data = packet_type.serialize()?;
        Ok(Self {
            length: data.len() as u32,
            data,
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        let length = self.length.to_be_bytes();

        let mut vec: Vec<u8> = Vec::new();
        vec.extend_from_slice(&length);
        vec.extend_from_slice(&self.data);
        vec
    }

    pub fn decode(buffer: &mut Vec<u8>) -> Result<Self, DecodeError> {
        if buffer.len() < 4 {
            return Err(DecodeError("Buffer is too small".to_string()));
        }

        let length = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
        if buffer.len() < length + 4 {
            return Err(DecodeError("Buffer is too small".to_string()));
        }

        buffer.drain(0..4);
        Ok(Self {
            length: length as u32,
            data: buffer.drain(..length).collect::<Vec<u8>>(),
        })
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

    #[test]
    fn should_create_new_packet() {
        let packet = Packet::new(PacketType::Connect).unwrap();
        assert_eq!(packet.length, 4);
        assert_eq!(packet.data, vec![0, 0, 0, 0]);
    }

    #[test]
    fn should_encode_packet() {
        let packet = Packet::new(PacketType::Connect).unwrap();
        assert_eq!(packet.encode(), vec![0, 0, 0, 4, 0, 0, 0, 0]);
    }

    #[test]
    fn should_encode_and_encode_packet() {
        let packet = Packet::new(PacketType::Connect).unwrap();
        assert_eq!(packet, Packet::decode(&mut packet.encode()).unwrap());
    }

    #[test]
    fn test_packet_decode_small_buffer() {
        assert!(Packet::decode(&mut vec![0, 0, 0]).is_err());
    }

    #[test]
    fn test_packet_decode_large_buffer() {
        assert!(Packet::decode(&mut vec![0, 0, 0, 4, 0]).is_err());
    }
}

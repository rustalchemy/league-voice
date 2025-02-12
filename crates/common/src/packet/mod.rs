pub mod error;
pub mod ids;
pub mod packet_type;
pub mod types;

pub use types::{audio::AudioPacket, connect::ConnectPacket, disconnect::DisconnectPacket};

use error::DecodeError;
use packet_type::PacketType;
use serde::{Deserialize, Serialize};

pub const MAX_PACKET_SIZE: usize = 1024;

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Packet {
    pub length: u32,
    pub packet_id: u8,
    pub data: Vec<u8>,
}

impl Packet {
    pub fn new<P: PacketType>(packet_type: P) -> Self {
        let data = P::encode(&packet_type);
        Self {
            length: data.len() as u32,
            packet_id: P::packet_id().to_u8(),
            data,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut vec: Vec<u8> = Vec::new();
        vec.extend_from_slice(&self.length.to_be_bytes());
        vec.extend_from_slice(&self.packet_id.to_be_bytes());
        vec.extend_from_slice(&self.data);
        vec
    }

    pub fn decode(buffer: &mut Vec<u8>) -> Result<Self, DecodeError> {
        if buffer.len() < 5 {
            return Err(DecodeError("Buffer is too small".to_string()));
        }

        let length = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
        if buffer.len() < length + 5 {
            return Err(DecodeError("Buffer is too small".to_string()));
        }

        buffer.drain(0..4);
        let packet_id = u8::from_be_bytes([buffer[0]]);
        buffer.drain(0..1);

        Ok(Self {
            length: length as u32,
            packet_id,
            data: buffer.drain(..length).collect::<Vec<u8>>(),
        })
    }
}

impl From<Packet> for Vec<u8> {
    fn from(packet: Packet) -> Self {
        packet.encode()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_create_new_packet() {
        let packet = Packet::new(ConnectPacket);
        assert_eq!(packet.length, 0);
        assert_eq!(packet.packet_id, 0);
        assert_eq!(packet.data, vec![]);

        let packet = Packet::new(DisconnectPacket);
        assert_eq!(packet.length, 0);
        assert_eq!(packet.packet_id, 1);
        assert_eq!(packet.data, vec![]);

        let packet = Packet::new(AudioPacket { track: vec![1] });
        assert_eq!(packet.length, 9);
        assert_eq!(packet.packet_id, 2);
        assert_eq!(packet.data, vec![1, 0, 0, 0, 0, 0, 0, 0, 1]);
    }

    #[test]
    fn should_encode_and_encode_packet() {
        let packet = Packet::new(ConnectPacket);
        assert_eq!(packet, Packet::decode(&mut packet.encode()).unwrap());
    }

    #[test]
    fn test_packet_decode_small_buffer() {
        assert!(Packet::decode(&mut vec![0, 0, 0]).is_err());
    }

    #[test]
    fn test_packet_decode_large_buffer() {
        assert!(Packet::decode(&mut vec![0, 0, 0, 4, 0, 0]).is_err());
    }

    #[test]
    fn test_packet_to_vec() {
        let packet = Packet::new(ConnectPacket);
        assert_eq!(packet.encode(), Vec::from(packet));
    }
}

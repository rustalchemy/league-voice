use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum PacketType {
    Connect,
    Disconnect,
    Audio(Vec<u8>),
}

impl PacketType {
    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        bincode::serialize(self).map_err(|e| e.to_string())
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        bincode::deserialize(data).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Packet {
    pub length: u32,
    pub data: Vec<u8>,
}

impl Packet {
    pub fn new(packet_type: PacketType) -> Self {
        let data = match packet_type.serialize() {
            Ok(data) => data,
            Err(e) => panic!("Failed to serialize packet type: {}", e),
        };

        Self {
            length: data.len() as u32,
            data,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, String> {
        let length = self.length.to_be_bytes();

        let mut vec: Vec<u8> = Vec::new();
        vec.extend_from_slice(&length);
        vec.extend_from_slice(&self.data);

        Ok(vec)
    }

    pub fn decode(buffer: &mut Vec<u8>) -> Result<Self, String> {
        if buffer.len() < 4 {
            return Err("Buffer is too small".to_string());
        }

        let length = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
        if buffer.len() < length + 4 {
            return Err("Buffer is too small".to_string());
        }

        println!("Length: {}", length);
        buffer.drain(0..4);
        let data = buffer.drain(..length).collect::<Vec<u8>>();
        // let data = buffer[4..length + 4].to_vec();
        Ok(Self {
            length: length as u32,
            data,
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
        let packet = Packet::new(PacketType::Connect);
        assert_eq!(packet.length, 4);
        assert_eq!(packet.data, vec![0, 0, 0, 0]);
    }

    #[test]
    fn should_encode_packet() {
        let packet = Packet::new(PacketType::Connect);
        let encoded = packet.encode().unwrap();
        assert_eq!(encoded, vec![0, 0, 0, 4, 0, 0, 0, 0]);
    }

    #[test]
    fn should_encode_and_encode_packet() {
        let packet = Packet::new(PacketType::Connect);
        assert_eq!(
            packet,
            Packet::decode(&mut packet.encode().unwrap()).unwrap()
        );
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

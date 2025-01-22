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

use crate::error::ClientError;

pub mod opus;

pub trait AudioCodec: Send + Sync {
    fn new() -> Result<Self, ClientError>
    where
        Self: Sized;

    fn update(&mut self, sample_rate: u32, channels: usize) -> Result<(), ClientError>;
    fn encode(&mut self, data: Vec<f32>) -> Result<Vec<u8>, ClientError>;
    fn decode(&mut self, data: Vec<u8>) -> Result<Vec<f32>, ClientError>;
}

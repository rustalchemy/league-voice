pub mod opus;

pub trait AudioCodec: Send + Sync {
    fn new() -> Self;
    fn encode(&self, data: Vec<f32>) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
    fn decode(&self, data: Vec<u8>) -> Result<Vec<f32>, Box<dyn std::error::Error>>;
}

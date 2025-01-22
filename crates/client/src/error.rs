use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("failed on io: {0}")]
    IoError(#[from] std::io::Error),
}

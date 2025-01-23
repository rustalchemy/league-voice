use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("failed on io: {0}")]
    IoError(#[from] std::io::Error),

    #[error("failed on bincode: {0}")]
    BincodeError(#[from] Box<bincode::ErrorKind>),
}

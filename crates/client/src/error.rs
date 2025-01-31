use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("make sure you have at least one host available")]
    NoHost,

    #[error("failed to create audio device")]
    NoDevice,

    #[error("failed to build stream: {0}")]
    BuildStreamError(#[from] cpal::BuildStreamError),

    #[error("failed to play stream: {0}")]
    PlayStreamError(#[from] cpal::PlayStreamError),

    #[error("invalid channel count")]
    InvalidChannelCount,

    #[error("failed within opus: {0}")]
    OpusError(#[from] opus::Error),

    #[error("buffer overflow: possible protocol error")]
    BufferOverflow,

    #[error("connection closed by peer")]
    ConnectionClosedByPeer,

    #[error("failed on io: {0}")]
    IoError(#[from] std::io::Error),

    #[error("failed on bincode: {0}")]
    BincodeError(#[from] Box<bincode::ErrorKind>),

    #[error("failed on std mpsc send f32: {0}")]
    StdSendErrorF32(#[from] std::sync::mpsc::SendError<Vec<f32>>),

    #[error("failed on tokio mpsc send u8: {0}")]
    TokioSendErrorU8(#[from] tokio::sync::mpsc::error::SendError<Vec<u8>>),
}

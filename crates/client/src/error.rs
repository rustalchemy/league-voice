use common::packet::Packet;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("failed to get host: {0}")]
    HostUnavailable(#[from] cpal::HostUnavailable),

    #[error("audio handler already started: {0}")]
    AudioHandlerAlreadyStarted(String),

    #[error("default stream config error: {0}")]
    DefaultStreamConfigError(#[from] cpal::DefaultStreamConfigError),

    #[error("failed to create audio stream: {0}")]
    DevicesError(#[from] cpal::DevicesError),

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

    #[error("failed on tokio mpsc send Packet: {0}")]
    TokioSendErrorPacket(#[from] tokio::sync::mpsc::error::SendError<Packet>),

    #[error("failed on tokio broadcast send Packet: {0}")]
    TokioSendErrorBroadcastPacket(#[from] tokio::sync::broadcast::error::SendError<Packet>),

    #[error("invalid packet")]
    InvalidPacket,

    #[error("codec not initialized")]
    CodecNotInitialized,

    #[error("failed to resample: {0}")]
    ResampleError(#[from] rubato::ResampleError),

    #[error("failed to create resampler: {0}")]
    ResamplerConstructionError(#[from] rubato::ResamplerConstructionError),

    #[error("invalid frame size")]
    InvalidFrameSize,

    #[error("poisoned lock")]
    PoisonedLock,
}

use std::fmt::Display;

use crate::error::ClientError;
use ::cpal::SupportedStreamConfig;
use serde::Serialize;
use tokio::sync::mpsc::{Receiver, Sender};

pub mod codec;
pub mod cpal;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Serialize)]
pub enum DeviceType {
    Input,
    Output,
}

impl Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceType::Input => write!(f, "Input"),
            DeviceType::Output => write!(f, "Output"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    name: String,
    device_type: DeviceType,
    active: bool,
    default: bool,

    #[serde(skip)]
    config: SupportedStreamConfig,
}

#[async_trait::async_trait]
pub trait AudioHandler: Send + Sync {
    async fn start(
        &self,
        input: Sender<Vec<u8>>,
        output: Receiver<Vec<u8>>,
    ) -> Result<(), ClientError>;
}

#[async_trait::async_trait]
pub trait DeviceHandler {
    fn get_devices(&self, device_type: DeviceType) -> Vec<DeviceInfo>;
    async fn set_active_device(&mut self, device_name: String) -> Result<(), ClientError>;
}

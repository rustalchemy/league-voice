use std::fmt::Display;

use crate::error::ClientError;
use ::cpal::{Device, SupportedStreamConfig};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender};

pub mod codec;
pub mod cpal;
pub mod cpal_device;
pub mod cpal_util;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
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

#[derive(Clone, Serialize)]
pub struct DeviceInfo {
    name: String,
    device_type: DeviceType,
    active: bool,
    default: bool,

    #[serde(skip)]
    config: SupportedStreamConfig,

    #[serde(skip)]
    device: Option<Device>,
}

#[async_trait::async_trait]
pub trait AudioHandler: Send + Sync + Sized {
    fn new() -> Result<Self, ClientError>;
    async fn start(
        &self,
        packet_sender: Sender<Vec<u8>>,
        packet_receiver: Receiver<Vec<u8>>,

        mic_rx: Receiver<Vec<f32>>,
        output_tx: std::sync::mpsc::Sender<Vec<f32>>,
    ) -> Result<(), ClientError>;
    async fn stop(&self) -> Result<(), ClientError>;
}

#[async_trait::async_trait]
pub trait DeviceHandler: Send + Sync + Sized {
    fn new() -> Result<Self, ClientError>;

    fn get_devices(&self, device_type: DeviceType) -> Vec<DeviceInfo>;

    async fn start_defaults(
        &mut self,
        mic_tx: Sender<Vec<f32>>,
        output_rx: std::sync::mpsc::Receiver<Vec<f32>>,
    ) -> Result<(), ClientError>;

    async fn set_active_device(
        &mut self,
        device_type: &DeviceType,
        device_name: String,
    ) -> Result<(), ClientError>;
}

use crate::error::ClientError;
use ::cpal::{Device, SupportedStreamConfig};
use codec::AudioCodec;
use common::packet::Packet;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};

pub mod codec;
pub mod cpal_device;
pub mod cpal_util;
pub mod processor;

#[derive(Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum DeviceType {
    Input,
    Output,
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

impl DeviceInfo {
    pub fn config(&self) -> SupportedStreamConfig {
        self.config.clone()
    }
}

#[async_trait::async_trait]
pub trait SoundProcessor: Send + Sync + Sized {
    type Codec: AudioCodec;
    fn new() -> Result<Self, ClientError>;
    async fn start(&self, mut mic_rx: Receiver<Vec<f32>>, packet_sender: Sender<Packet>);
    async fn stop(&self);
    fn get_codec(&self) -> Arc<Mutex<Self::Codec>>;
}

pub trait DeviceHandler: Send + Sync + Sized {
    fn new() -> Result<Self, ClientError>;

    fn get_devices(&self, device_type: DeviceType) -> Vec<DeviceInfo>;
    fn get_active_device(&self, device_type: DeviceType) -> Option<DeviceInfo>;

    fn start_actives(
        &mut self,
        mic_tx: Sender<Vec<f32>>,
        output_rx: std::sync::mpsc::Receiver<Vec<f32>>,
    ) -> Result<(), ClientError>;

    fn set_active_device(
        &mut self,
        device_type: &DeviceType,
        device_name: String,
    ) -> Result<(), ClientError>;

    fn stop(&mut self) -> Result<(), ClientError>;
}

use super::{
    cpal_util::{
        get_device_config, get_host, init_device_type, setup_input_stream, setup_output_stream,
    },
    DeviceHandler, DeviceInfo, DeviceType,
};
use crate::error::ClientError;
use cpal::{traits::HostTrait, Stream};
use tokio::sync::mpsc::Sender;

#[allow(dead_code)]
struct SendStream(Option<Stream>);

// Hack to implement Send and Sync for SendStream
// This is necessary because the Stream type from cpal does not implement Send and Sync
// https://github.com/RustAudio/cpal/issues/818
// Safety: SendStream is not actually used, it is only function to hold a pointer
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}

pub struct CpalDeviceHandler {
    devices: Vec<DeviceInfo>,

    input_stream: SendStream,
    output_stream: SendStream,
}

#[async_trait::async_trait]
impl DeviceHandler for CpalDeviceHandler {
    fn new() -> Result<Self, ClientError> {
        let host = get_host()?;

        let input_devices = init_device_type(DeviceType::Input, &host)?;
        let output_devices = init_device_type(DeviceType::Output, &host)?;

        let devices = input_devices
            .into_iter()
            .chain(output_devices.into_iter())
            .collect();

        Ok(Self {
            devices,
            input_stream: SendStream(None),
            output_stream: SendStream(None),
        })
    }

    fn get_devices(&self, device_type: DeviceType) -> Vec<DeviceInfo> {
        self.devices
            .iter()
            .filter(|device| device.device_type == device_type)
            .cloned()
            .collect()
    }

    async fn start_defaults(
        &mut self,
        mic_tx: Sender<Vec<f32>>,
        output_rx: std::sync::mpsc::Receiver<Vec<f32>>,
    ) -> Result<(), ClientError> {
        let input_devices = self.get_devices(DeviceType::Input);
        let output_devices = self.get_devices(DeviceType::Output);

        let input_device = input_devices
            .iter()
            .find(|device| device.active)
            .ok_or(ClientError::NoDevice)?;

        let output_device = output_devices
            .iter()
            .find(|device| device.active)
            .ok_or(ClientError::NoDevice)?;

        let host = get_host()?;
        let mut devices = host.devices()?;

        let (input_device, input_config) =
            get_device_config(&input_device.name, &input_devices, &mut devices)?;
        let (output_device, output_config) =
            get_device_config(&output_device.name, &output_devices, &mut devices)?;

        self.input_stream = SendStream(Some(setup_input_stream(
            &input_device,
            &input_config.config,
            mic_tx,
        )?));
        self.output_stream = SendStream(Some(setup_output_stream(
            &output_device,
            &output_config.config,
            output_rx,
        )?));

        self.devices = input_devices
            .into_iter()
            .chain(output_devices.into_iter())
            .collect();

        Ok(())
    }

    async fn set_active_device(
        &mut self,
        device_type: DeviceType,
        device_name: String,
    ) -> Result<(), ClientError> {
        for device in self.devices.iter_mut() {
            if device.device_type != device_type {
                continue;
            }
            device.active = false;

            if device.name != device_name {
                continue;
            }
            device.active = true;
        }

        Ok(())
    }
}

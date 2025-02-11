use super::{
    cpal_util::{
        get_device_config, get_host, init_device_type, setup_input_stream, setup_output_stream,
    },
    DeviceHandler, DeviceInfo, DeviceType,
};
use crate::error::ClientError;
use cpal::{traits::StreamTrait, Stream};
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
    input_devices: Vec<DeviceInfo>,
    output_devices: Vec<DeviceInfo>,

    input_stream: SendStream,
    output_stream: SendStream,
}

#[async_trait::async_trait]
impl DeviceHandler for CpalDeviceHandler {
    fn new() -> Result<Self, ClientError> {
        let host: cpal::Host = get_host()?;

        let input_devices = init_device_type(DeviceType::Input, &host)?;
        let output_devices = init_device_type(DeviceType::Output, &host)?;

        Ok(Self {
            input_devices,
            output_devices,
            input_stream: SendStream(None),
            output_stream: SendStream(None),
        })
    }

    fn get_devices(&self, device_type: DeviceType) -> Vec<DeviceInfo> {
        let devices: Vec<DeviceInfo> = self
            .input_devices
            .clone()
            .into_iter()
            .chain(self.output_devices.clone().into_iter())
            .collect();

        devices
            .iter()
            .filter(|device| device.device_type == device_type)
            .cloned()
            .collect()
    }

    fn get_active_device(&self, device_type: DeviceType) -> Option<DeviceInfo> {
        match device_type {
            DeviceType::Input => self
                .input_devices
                .iter()
                .find(|device| device.active)
                .cloned(),
            DeviceType::Output => self
                .output_devices
                .iter()
                .find(|device| device.active)
                .cloned(),
        }
    }

    async fn start_actives(
        &mut self,
        mic_tx: Sender<Vec<f32>>,
        output_rx: std::sync::mpsc::Receiver<Vec<f32>>,
    ) -> Result<(), ClientError> {
        println!("Starting default devices");
        let input_device = self
            .input_devices
            .iter()
            .find(|device| device.active)
            .ok_or(ClientError::NoDevice)?;

        let output_device = self
            .output_devices
            .iter()
            .find(|device| device.active)
            .ok_or(ClientError::NoDevice)?;

        let input_config = get_device_config(&input_device.name, &self.input_devices.clone())?;
        let output_config = get_device_config(&output_device.name, &self.output_devices.clone())?;

        let input_device = input_device.device.as_ref().unwrap();
        let output_device = output_config.device.as_ref().unwrap();

        self.input_stream = SendStream(Some(setup_input_stream(
            &input_device,
            &input_config,
            mic_tx,
        )?));
        self.output_stream = SendStream(Some(setup_output_stream(
            &output_device,
            &output_config,
            output_rx,
        )?));

        Ok(())
    }

    async fn set_active_device(
        &mut self,
        device_type: &DeviceType,
        device_name: String,
    ) -> Result<(), ClientError> {
        // set other devices to inactive
        match device_type {
            DeviceType::Input => {
                self.input_devices.iter_mut().for_each(|device| {
                    device.active = false;
                });
            }
            DeviceType::Output => {
                self.output_devices.iter_mut().for_each(|device| {
                    device.active = false;
                });
            }
        }

        let device = match device_type {
            DeviceType::Input => self
                .input_devices
                .iter_mut()
                .find(|device| device.name == device_name),
            DeviceType::Output => self
                .output_devices
                .iter_mut()
                .find(|device| device.name == device_name),
        };

        let device = match device {
            Some(device) => device,
            None => {
                return Err(ClientError::NoDevice);
            }
        };

        device.active = true;

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), ClientError> {
        if let Some(input_stream) = self.input_stream.0.take() {
            match input_stream.pause() {
                Ok(_) => {}
                Err(e) => {
                    println!("Error pausing input stream: {:?}", e);
                }
            };
        }

        if let Some(output_stream) = self.output_stream.0.take() {
            match output_stream.pause() {
                Ok(_) => {}
                Err(e) => {
                    println!("Error pausing output stream: {:?}", e);
                }
            };
        }

        Ok(())
    }
}

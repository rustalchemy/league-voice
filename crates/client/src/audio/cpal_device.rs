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

    fn start_actives(
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

    fn set_active_device(
        &mut self,
        device_type: &DeviceType,
        device_name: String,
    ) -> Result<(), ClientError> {
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

    fn stop(&mut self) -> Result<(), ClientError> {
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

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread::sleep, time::Duration};

    use super::*;
    use crate::audio::cpal_util::get_host_devices;
    use tokio::sync::{mpsc, Mutex};

    #[tokio::test]
    async fn test_cpal_device_handler() {
        let mut handler = CpalDeviceHandler::new().unwrap();
        let host = get_host().unwrap();

        let input_devices = get_host_devices(&DeviceType::Input, &host).unwrap();
        let output_devices = get_host_devices(&DeviceType::Output, &host).unwrap();

        let input_device = input_devices.first().unwrap();
        let output_device = output_devices.first().unwrap();

        handler
            .set_active_device(&DeviceType::Input, input_device.name.clone())
            .unwrap();
        handler
            .set_active_device(&DeviceType::Output, output_device.name.clone())
            .unwrap();

        let active_input_device = handler.get_active_device(DeviceType::Input).unwrap();
        let active_output_device = handler.get_active_device(DeviceType::Output).unwrap();

        assert_eq!(active_input_device.name, input_device.name);
        assert_eq!(active_output_device.name, output_device.name);
    }

    #[tokio::test]
    async fn test_cpal_device_handler_start() {
        let mut handler = CpalDeviceHandler::new().unwrap();
        let host = get_host().unwrap();

        let input_devices = get_host_devices(&DeviceType::Input, &host).unwrap();
        let output_devices = get_host_devices(&DeviceType::Output, &host).unwrap();

        let input_device = input_devices.first().unwrap();
        let output_device = output_devices.first().unwrap();

        handler
            .set_active_device(&DeviceType::Input, input_device.name.clone())
            .unwrap();
        handler
            .set_active_device(&DeviceType::Output, output_device.name.clone())
            .unwrap();

        let (mic_tx, mut mic_rx) = mpsc::channel(10);
        let (_, output_rx) = std::sync::mpsc::channel();

        let handler = Arc::new(Mutex::new(handler));
        tokio::spawn(async move {
            handler
                .clone()
                .lock()
                .await
                .start_actives(mic_tx, output_rx)
                .unwrap();
        });

        assert_eq!(
            mic_rx.recv().await.unwrap().len(),
            (input_device.config.sample_rate().0 * input_device.config.channels() as u32 / 100)
                .try_into()
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_cpal_device_handler_stop() {
        let mut handler = CpalDeviceHandler::new().unwrap();
        let host = get_host().unwrap();

        let input_devices = get_host_devices(&DeviceType::Input, &host).unwrap();
        let output_devices = get_host_devices(&DeviceType::Output, &host).unwrap();

        let input_device = input_devices.first().unwrap();
        let output_device = output_devices.first().unwrap();

        handler
            .set_active_device(&DeviceType::Input, input_device.name.clone())
            .unwrap();
        handler
            .set_active_device(&DeviceType::Output, output_device.name.clone())
            .unwrap();

        let (mic_tx, _) = mpsc::channel(10);
        let (_, output_rx) = std::sync::mpsc::channel();
        let handler = Arc::new(Mutex::new(handler));
        let handler_cloned = handler.clone();
        tokio::spawn(async move {
            let mut handler = handler_cloned.lock().await;
            handler.start_actives(mic_tx, output_rx).unwrap();
            handler.stop().unwrap();
        });

        sleep(Duration::from_millis(150));

        let handler = handler.lock().await;
        assert_eq!(handler.input_stream.0.is_none(), true);
        assert_eq!(handler.output_stream.0.is_none(), true);
    }

    #[tokio::test]
    async fn test_get_devices() {
        let handler = CpalDeviceHandler::new().unwrap();
        let host = get_host().unwrap();

        assert_eq!(
            handler.get_devices(DeviceType::Input).len(),
            get_host_devices(&DeviceType::Input, &host).unwrap().len()
        );
        assert_eq!(
            handler.get_devices(DeviceType::Output).len(),
            get_host_devices(&DeviceType::Output, &host).unwrap().len()
        );
    }
}

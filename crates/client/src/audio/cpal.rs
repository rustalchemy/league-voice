use super::{codec::AudioCodec, AudioHandler, DeviceInfo, DeviceType};
use crate::error::ClientError;
use common::packet::{packet_type::PacketType, AudioPacket, Packet};
use cpal::SupportedStreamConfig;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Devices, Stream,
};
use std::sync::Arc;
use std::u32;
use tokio::{
    select,
    sync::{
        mpsc::{self, Receiver},
        Mutex,
    },
};

#[allow(dead_code)]
struct SendStream(Option<Stream>);

// Hack to implement Send and Sync for SendStream
// This is necessary because the Stream type from cpal does not implement Send and Sync
// https://github.com/RustAudio/cpal/issues/818
// Safety: SendStream is not actually used, it is only function to hold a pointer
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}

pub struct CpalAudioHandler<Codec: AudioCodec> {
    codec: Arc<Codec>,

    mic_rx: Arc<Mutex<Receiver<Vec<f32>>>>,
    output_tx: Arc<Mutex<std::sync::mpsc::Sender<Vec<f32>>>>,

    input_stream: SendStream,
    output_stream: SendStream,

    devices: Vec<DeviceInfo>,
}

impl<Codec: AudioCodec> CpalAudioHandler<Codec> {
    fn get_host() -> Result<cpal::Host, ClientError> {
        let host_id = match cpal::available_hosts().into_iter().next() {
            Some(host) => host,
            None => {
                println!("No host setup device found");
                return Err(ClientError::NoHost);
            }
        };
        Ok(cpal::host_from_id(host_id)?)
    }

    fn get_host_devices(
        device_type: &DeviceType,
        host: &cpal::Host,
    ) -> Result<Vec<DeviceInfo>, ClientError> {
        let host_devices = match device_type {
            DeviceType::Input => host.input_devices()?,
            DeviceType::Output => host.output_devices()?,
        };

        let default_device = match device_type {
            DeviceType::Input => host.default_input_device(),
            DeviceType::Output => host.default_output_device(),
        };

        let default_device = match default_device {
            Some(device) => device,
            None => {
                println!("No default device found");
                return Err(ClientError::NoDevice);
            }
        };
        let default_name = default_device.name().unwrap_or_default();

        let mut devices = Vec::new();
        for device in host_devices {
            let config = match device_type {
                DeviceType::Input => device.default_input_config()?,
                DeviceType::Output => device.default_output_config()?,
            };

            let name = device.name().unwrap_or_default();
            let device_info = DeviceInfo {
                name: name.clone(),
                device_type: device_type.clone(),
                active: false,
                default: name == default_name,
                config,
            };
            devices.push(device_info);
        }
        Ok(devices)
    }

    fn find_default(
        device_name: &str,
        device_infos: &Vec<DeviceInfo>,
        devices: &mut Devices,
    ) -> Result<(SupportedStreamConfig, Device), ClientError> {
        let device_info = device_infos
            .iter()
            .find(|device| device.name == device_name);
        let device_info = match device_info {
            Some(device) => device,
            None => {
                return Err(ClientError::NoDevice);
            }
        };

        let config = device_info.config.clone();
        let device = match devices.find(|device| device.name().unwrap_or_default() == device_name) {
            Some(device) => device,
            None => {
                return Err(ClientError::NoDevice);
            }
        };
        Ok((config, device))
    }

    fn setup_input_stream(
        device: &Device,
        config: &SupportedStreamConfig,
        mic_tx: mpsc::Sender<Vec<f32>>,
    ) -> Result<Stream, ClientError> {
        let input_stream = device.build_input_stream(
            &config.config(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let _ = mic_tx.try_send(data.to_vec());
            },
            |err| eprintln!("Input stream error: {:?}", err),
            None,
        )?;
        Ok(input_stream)
    }

    fn setup_output_stream(
        device: &Device,
        config: &SupportedStreamConfig,
        output_rx: std::sync::mpsc::Receiver<Vec<f32>>,
    ) -> Result<Stream, ClientError> {
        let output_stream = device.build_output_stream(
            &config.config(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if let Ok(decoded_data) = output_rx.try_recv() {
                    let len_to_copy = decoded_data.len().min(data.len());
                    println!("Copying {} samples", len_to_copy);
                    data[..len_to_copy].copy_from_slice(&decoded_data[..len_to_copy]);
                    if len_to_copy < data.len() {
                        data[len_to_copy..].fill(0.0);
                    }
                } else {
                    data.fill(0.0);
                }
            },
            |err| eprintln!("Output stream error: {:?}", err),
            None,
        )?;
        Ok(output_stream)
    }

    fn init_device_type(
        device_type: DeviceType,
        host: &cpal::Host,
        mic_tx: Option<mpsc::Sender<Vec<f32>>>,
        output_rx: Option<std::sync::mpsc::Receiver<Vec<f32>>>,
    ) -> Result<(Stream, Vec<DeviceInfo>), ClientError> {
        let mut devices = host.devices()?;
        let devices_info = Self::get_host_devices(&device_type, &host)?;
        let default_device = devices_info.iter().find(|device| device.default).unwrap();

        let (config, device) =
            Self::find_default(&default_device.name, &devices_info, &mut devices)?;

        let stream = match device_type {
            DeviceType::Input => Self::setup_input_stream(&device, &config, mic_tx.unwrap())?,
            DeviceType::Output => Self::setup_output_stream(&device, &config, output_rx.unwrap())?,
        };

        println!("Starting {} stream", device_type);
        println!("Config: {:?}", config);
        println!("Sample rate: {:?}", config.sample_rate().0);
        println!("Buffer size: {:?}", config.buffer_size());
        println!("Sample format: {:?}", config.sample_format());
        println!("Channels: {:?}", config.channels());
        println!("Device name: {:?}", device.name());
        println!();

        stream.play()?;

        Ok((stream, devices_info))
    }

    pub fn new() -> Result<Self, ClientError> {
        let (mic_tx, mic_rx) = mpsc::channel::<Vec<f32>>(20);
        let (output_tx, output_rx) = std::sync::mpsc::channel::<Vec<f32>>();

        let host = Self::get_host()?;

        let (microphone_stream, input_devices) =
            Self::init_device_type(DeviceType::Input, &host, Some(mic_tx), None)?;

        let (output_stream, output_devices) =
            Self::init_device_type(DeviceType::Output, &host, None, Some(output_rx))?;

        let codec = Codec::new(48000, 1)?;
        Ok(CpalAudioHandler {
            codec: Arc::new(codec),

            mic_rx: Arc::new(Mutex::new(mic_rx)),
            output_tx: Arc::new(Mutex::new(output_tx)),

            input_stream: SendStream(Some(microphone_stream)),
            output_stream: SendStream(Some(output_stream)),

            devices: input_devices
                .into_iter()
                .chain(output_devices.into_iter())
                .collect(),
        })
    }
}

#[async_trait::async_trait]
impl<Codec: AudioCodec + 'static> AudioHandler for CpalAudioHandler<Codec> {
    async fn start(
        &self,
        input_tx: mpsc::Sender<Vec<u8>>,
        mut output_rx: mpsc::Receiver<Vec<u8>>,
    ) -> Result<(), ClientError> {
        let (audio_tx, mut audio_rx) = mpsc::channel::<Vec<u8>>(20);

        let codec = self.codec.clone();
        let mic_rx = self.mic_rx.clone();
        let microphone_handle = tokio::spawn(async move {
            while let Some(audio_samples) = mic_rx.lock().await.recv().await {
                if let Ok(encoded_data) = codec.encode(audio_samples) {
                    let _ = audio_tx.send(encoded_data).await;
                }
            }
            Ok(())
        });

        let audio_packets_handle = tokio::spawn(async move {
            while let Some(track) = audio_rx.recv().await {
                if let Ok(packet) = Packet::new(AudioPacket { track }) {
                    let encoded_packet = packet.encode();
                    let _ = input_tx.send(encoded_packet).await;
                }
            }
            Ok(())
        });

        let codec = self.codec.clone();
        let output_tx = self.output_tx.clone();
        let codec_handle = tokio::spawn(async move {
            let output_tx = output_tx.lock().await;
            while let Some(audio_samples) = output_rx.recv().await {
                let audio_packet = AudioPacket::decode(&audio_samples)?;
                if let Ok(decoded_data) = codec.decode(audio_packet.track) {
                    output_tx.send(decoded_data)?;
                }
            }
            Ok(())
        });

        select! {
            Ok(result) = microphone_handle => {
                result
            },
            Ok(result) = audio_packets_handle => {
                result
            },
            Ok(result) = codec_handle => {
                result
            }
        }
    }

    fn get_devices(&self, device_type: DeviceType) -> Vec<DeviceInfo> {
        self.devices
            .iter()
            .filter(|device| device.device_type == device_type)
            .cloned()
            .collect()
    }

    async fn set_active_device(
        &mut self,
        device_type: DeviceType,
        device_name: String,
    ) -> Result<(), ClientError> {
        let host = Self::get_host()?;
        let mut devices = host.devices()?;

        let devices_info = Self::get_host_devices(&device_type, &host)?;
        let (config, device) = Self::find_default(&device_name, &devices_info, &mut devices)?;

        match device_type {
            DeviceType::Input => {
                let (mic_tx, mic_rx) = mpsc::channel::<Vec<f32>>(20);

                self.input_stream =
                    SendStream(Some(Self::setup_input_stream(&device, &config, mic_tx)?));
                self.mic_rx = Arc::new(Mutex::new(mic_rx));
            }
            DeviceType::Output => {
                let (output_tx, output_rx) = std::sync::mpsc::channel::<Vec<f32>>();

                self.output_stream = SendStream(Some(Self::setup_output_stream(
                    &device, &config, output_rx,
                )?));
                self.output_tx = Arc::new(Mutex::new(output_tx));
            }
        }

        self.devices = devices_info;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::codec::opus::OpusAudioCodec;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_cpal_audio_handler() {
        let audio_handler = CpalAudioHandler::<OpusAudioCodec>::new().unwrap();

        let (tx, rx) = mpsc::channel(1000);
        let (tx_2, mut rx_2) = mpsc::channel(1000);

        let audio_handler_handle = tokio::spawn(async move { audio_handler.start(tx, rx).await });

        let sender_handle = tokio::spawn(async move {
            let packet = Packet::new(AudioPacket {
                track: vec![0; 960],
            })
            .unwrap();
            let encoded_packet = packet.encode();
            for _ in 0..100 {
                tx_2.send(encoded_packet.clone()).await.unwrap();
            }

            sleep(Duration::from_micros(100)).await;

            Ok::<(), ()>(())
        });

        let receiver_handle = tokio::spawn(async move {
            let mut count = 0;
            while let Some(_) = rx_2.recv().await {
                count += 1;
                if count == 100 {
                    break;
                }
            }
            return Ok::<i32, ()>(count);
        });

        select! {
            _ = audio_handler_handle => {
                panic!("Audio handler exited unexpectedly");
            },
            _ = sender_handle => {
                panic!("Sender exited unexpectedly");
            },
            Ok(result) = receiver_handle => {
                assert_eq!(result.unwrap(), 100, "Receiver did not receive all packets");
            }
        };
    }
}

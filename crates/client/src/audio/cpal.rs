use super::{codec::AudioCodec, AudioHandler};
use crate::error::ClientError;
use common::packet::{packet_type::PacketType, AudioPacket, Packet};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Stream,
};
use std::sync::Arc;
use tokio::{
    select,
    sync::{
        mpsc::{self, Receiver},
        Mutex,
    },
};

#[allow(dead_code)]
struct SendStream(Stream);

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

    _input_stream: SendStream,
    _output_stream: SendStream,
}

impl<Codec: AudioCodec> CpalAudioHandler<Codec> {
    pub fn new() -> Result<Self, ClientError> {
        let host = cpal::default_host();
        let current_input_device = match host.default_input_device() {
            Some(device) => device,
            None => return Err(ClientError::NoDevice),
        };

        let current_input_config = match current_input_device.default_input_config() {
            Ok(config) => config,
            Err(err) => return Err(ClientError::DeviceConfig(err.to_string())),
        };

        let current_output_device = match host.default_output_device() {
            Some(device) => device,
            None => return Err(ClientError::NoDevice),
        };

        let current_output_config = match current_output_device.default_output_config() {
            Ok(config) => config,
            Err(err) => return Err(ClientError::DeviceConfig(err.to_string())),
        };

        let (mic_tx, mic_rx) = mpsc::channel::<Vec<f32>>(20);
        let (output_tx, output_rx) = std::sync::mpsc::channel::<Vec<f32>>();

        let microphone_stream: Stream = current_input_device.build_input_stream(
            &current_input_config.config(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let _ = mic_tx.try_send(data.to_vec());
            },
            |err| eprintln!("Input stream error: {:?}", err),
            None,
        )?;

        let output_stream = current_output_device.build_output_stream(
            &current_output_config.config(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if let Ok(decoded_data) = output_rx.try_recv() {
                    let len_to_copy = decoded_data.len().min(data.len());
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

        microphone_stream.play()?;
        output_stream.play()?;

        Ok(CpalAudioHandler {
            codec: Arc::new(Codec::new()?),

            mic_rx: Arc::new(Mutex::new(mic_rx)),
            output_tx: Arc::new(Mutex::new(output_tx)),

            _input_stream: SendStream(microphone_stream),
            _output_stream: SendStream(output_stream),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::codec::opus::OpusAudioCodec;

    #[tokio::test]
    async fn test_cpal_audio_handler() {
        let audio_handler = CpalAudioHandler::<OpusAudioCodec>::new().unwrap();
        let (tx, rx) = mpsc::channel(1000);
        let _ = audio_handler.start(tx, rx).await;
    }
}

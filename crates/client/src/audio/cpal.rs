use super::{codec::AudioCodec, AudioHandler};
use crate::error::ClientError;
use common::packet::{packet_type::PacketType, AudioPacket, Packet};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Sample, Stream,
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
    pub fn new() -> Self {
        let channels = 2;
        let host = cpal::default_host();

        let current_input_device = host
            .default_input_device()
            .expect("Failed to get default input device");

        let current_input_config = current_input_device
            .default_input_config()
            .expect("Failed to get default input config");

        let current_output_device = host
            .default_output_device()
            .expect("Failed to get default output device");

        let current_output_config = current_output_device
            .default_output_config()
            .expect("Failed to get default output config");

        let input_sample_format = current_input_config.sample_format();
        let putput_sample_format = current_output_config.sample_format();

        println!("Microphone Device: {:?}", current_input_device.name());
        println!("Speaker Device: {:?}", current_output_device.name());
        println!("Default Input Config: {:?}", current_input_config);
        println!("Default Output Config: {:?}", current_output_config);
        println!("Input Sample Format: {:?}", input_sample_format);
        println!("Output Sample Format: {:?}", putput_sample_format);

        let sample_rate = current_input_config.sample_rate().0 as u32;
        let channels = current_input_config.channels() as usize;
        let frame_size = (sample_rate / 1000) * 20;
        let frame_size_per_channel = frame_size as usize * channels;

        println!(
            "Sample rate: {}, Channels: {}, Frame size: {}",
            sample_rate, channels, frame_size
        );
        println!("Frame size per channel: {}", frame_size_per_channel);

        let (mic_tx, mic_rx) = mpsc::channel::<Vec<f32>>(1000);
        let (output_tx, output_rx) = std::sync::mpsc::channel::<Vec<f32>>();

        let microphone_stream: Stream = current_input_device
            .build_input_stream(
                &current_input_config.config(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let _ = mic_tx.try_send(data.to_vec());
                },
                |err| eprintln!("Input stream error: {:?}", err),
                None,
            )
            .expect("Failed to build input stream");

        microphone_stream
            .play()
            .expect("Failed to play input stream");

        // let channels = current_output_config.channels() as usize;

        let output_stream = current_output_device
            .build_output_stream(
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
            )
            .expect("Failed to build output stream");

        output_stream.play().expect("Failed to play output stream");

        CpalAudioHandler {
            codec: Arc::new(Codec::new()),

            mic_rx: Arc::new(Mutex::new(mic_rx)),
            output_tx: Arc::new(Mutex::new(output_tx)),

            _input_stream: SendStream(microphone_stream),
            _output_stream: SendStream(output_stream),
        }
    }
}

#[async_trait::async_trait]
impl<Codec: AudioCodec + 'static> AudioHandler for CpalAudioHandler<Codec> {
    async fn retrieve(
        &self,
        input_tx: mpsc::Sender<Vec<u8>>,
        mut output_rx: mpsc::Receiver<Vec<u8>>,
    ) -> Result<(), ClientError> {
        let (audio_tx, mut audio_rx) = mpsc::channel::<Vec<u8>>(1000);

        // retrieve audio samples from the microphone
        let codec = self.codec.clone();
        let mic_rx = self.mic_rx.clone();
        let microphone_handle = tokio::spawn(async move {
            while let Some(audio_samples) = mic_rx.lock().await.recv().await {
                let encoded_data = match codec.encode(audio_samples) {
                    Ok(data) => data,
                    Err(err) => {
                        eprintln!("Failed to encode audio samples: {:?}", err);
                        continue;
                    }
                };

                let _ = audio_tx.send(encoded_data).await;
            }
            Ok(())
        });

        // send audio packets to the server
        let audio_handle = tokio::spawn(async move {
            while let Some(track) = audio_rx.recv().await {
                let packet = match Packet::new(AudioPacket { track }) {
                    Ok(packet) => packet,
                    Err(err) => {
                        eprintln!("Failed to encode audio packet: {:?}", err);
                        continue;
                    }
                };

                let _ = input_tx.send(packet.encode()).await;
            }
            Ok(())
        });

        //send audio to output stream
        let codec = self.codec.clone();
        let output_tx = self.output_tx.clone();
        let output_handle = tokio::spawn(async move {
            let output_tx = output_tx.lock().await;
            while let Some(audio_samples) = output_rx.recv().await {
                let audio_packet = AudioPacket::decode(&audio_samples)?;
                let decoded_data = match codec.decode(audio_packet.track) {
                    Ok(data) => data,
                    Err(err) => {
                        eprintln!("Failed to decode audio samples: {:?}", err);
                        continue;
                    }
                };

                match output_tx.send(decoded_data) {
                    Ok(_) => {}
                    Err(err) => eprintln!("Failed to send audio samples: {:?}", err),
                }
            }

            Ok(())
        });

        select! {
            Ok(result) = microphone_handle => {
                result
            },
            Ok(result) = audio_handle => {
                result
            },
            Ok(result) = output_handle => {
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
        let audio_handler = CpalAudioHandler::<OpusAudioCodec>::new();
        let (tx, rx) = mpsc::channel(1000);
        let _ = audio_handler.retrieve(tx, rx).await;
    }
}

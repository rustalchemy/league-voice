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

    _input_stream: SendStream,
    _output_stream: SendStream,
}

impl<Codec: AudioCodec> CpalAudioHandler<Codec> {
    #[cfg(not(tarpaulin_include))]
    pub fn setup_host(
        mic_tx: mpsc::Sender<Vec<f32>>,
        output_rx: std::sync::mpsc::Receiver<Vec<f32>>,
    ) -> Result<(u32, Option<Stream>, Option<Stream>), ClientError> {
        use cpal::{SampleRate, SupportedStreamConfig};

        let hosts = match cpal::available_hosts().into_iter().next() {
            Some(host) => host,
            None => {
                println!("No host setup device found");
                return Err(ClientError::NoHost);
            }
        };

        let host = match cpal::host_from_id(hosts) {
            Ok(host) => Some(host),
            Err(err) => {
                println!("Cannot create host: {:?}", err);
                None
            }
        };

        let host = match host {
            Some(host) => host,
            None => {
                println!("No host device found");
                return Err(ClientError::NoDevice);
            }
        };

        let current_input_device = match host.default_input_device() {
            Some(device) => device,
            None => {
                println!("No input device found");
                return Err(ClientError::NoDevice);
            }
        };

        let current_input_config = SupportedStreamConfig::new(
            1,
            SampleRate(48000),
            cpal::SupportedBufferSize::Range { min: 0, max: 960 },
            cpal::SampleFormat::F32,
        );

        let current_output_device = match host.default_output_device() {
            Some(device) => device,
            None => {
                println!("No output device found");
                return Err(ClientError::NoDevice);
            }
        };

        let microphone_stream: Stream = current_input_device.build_input_stream(
            &current_input_config.config(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let _ = mic_tx.try_send(data.to_vec());
            },
            |err| eprintln!("Input stream error: {:?}", err),
            None,
        )?;

        let output_stream = current_output_device.build_output_stream(
            &current_input_config.config(),
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

        Ok((
            current_input_config.sample_rate().0,
            Some(microphone_stream),
            Some(output_stream),
        ))
    }

    #[cfg(not(tarpaulin_include))]
    pub fn new() -> Result<Self, ClientError> {
        let (mic_tx, mic_rx) = mpsc::channel::<Vec<f32>>(20);
        let (output_tx, output_rx) = std::sync::mpsc::channel::<Vec<f32>>();

        let streams = match Self::setup_host(mic_tx, output_rx) {
            Ok(sample_rate) => sample_rate,
            Err(err) => match err {
                ClientError::NoDevice => (48000, None, None),
                _ => {
                    return Err(err);
                }
            },
        };

        Ok(CpalAudioHandler {
            codec: Arc::new(Codec::new(streams.0, 1)?),

            mic_rx: Arc::new(Mutex::new(mic_rx)),
            output_tx: Arc::new(Mutex::new(output_tx)),

            _input_stream: SendStream(streams.1),
            _output_stream: SendStream(streams.2),
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

        let (audio_handler_result, sender_result, receiver_result) =
            tokio::join!(audio_handler_handle, sender_handle, receiver_handle);
        match (audio_handler_result, sender_result, receiver_result) {
            (Ok(_), Ok(_), Ok(count)) => {
                assert_eq!(count, Ok(100));
            }
            _ => panic!("Expected all futures to complete successfully"),
        }
    }
}

use super::{codec::AudioCodec, AudioHandler};
use crate::error::ClientError;
use common::packet::{AudioPacket, Packet};
use std::sync::Arc;
use std::u32;
use tokio::{
    select,
    sync::{
        mpsc::{self},
        Mutex,
    },
};

pub struct CpalAudioHandler<Codec: AudioCodec> {
    codec: Arc<Mutex<Codec>>,

    stop_tx: mpsc::Sender<()>,
    stop_rx: Arc<Mutex<mpsc::Receiver<()>>>,
}

impl<Codec: AudioCodec> CpalAudioHandler<Codec> {}

#[async_trait::async_trait]
impl<Codec: AudioCodec + 'static> AudioHandler for CpalAudioHandler<Codec> {
    type Codec = Codec;

    fn new() -> Result<Self, ClientError> {
        let (stop_tx, stop_rx) = mpsc::channel::<()>(1);

        Ok(CpalAudioHandler {
            codec: Arc::new(Mutex::new(Codec::new()?)),
            stop_tx,
            stop_rx: Arc::new(Mutex::new(stop_rx)),
        })
    }

    async fn start(
        &self,
        mut mic_rx: mpsc::Receiver<Vec<f32>>,
        packet_sender: mpsc::Sender<Packet>,
    ) -> Result<(), ClientError> {
        let (audio_tx, mut audio_rx) = mpsc::channel::<Vec<u8>>(20);

        let codec = self.codec.clone();
        let microphone_handle = tokio::spawn(async move {
            while let Some(audio_samples) = mic_rx.recv().await {
                let codec = codec.lock().await;
                if let Ok(encoded_data) = codec.encode(audio_samples) {
                    let _ = audio_tx.send(encoded_data).await;
                }
            }
            Ok(())
        });

        let audio_packets_handle = tokio::spawn(async move {
            while let Some(track) = audio_rx.recv().await {
                if let Ok(packet) = Packet::new(AudioPacket { track }) {
                    let _ = packet_sender.send(packet).await;
                }
            }
            Ok(())
        });

        let stop_rx = self.stop_rx.clone();
        let stop_handle: tokio::task::JoinHandle<Result<(), ClientError>> =
            tokio::spawn(async move {
                let mut stop_rx = stop_rx.lock().await;
                stop_rx.recv().await;
                Ok(())
            });

        select! {
            Ok(result) = microphone_handle => {
                result
            },
            Ok(result) = audio_packets_handle => {
                result
            },
            Ok(result) = stop_handle => {
                result
            }
        }
    }

    async fn stop(&self) -> Result<(), ClientError> {
        let _ = self.stop_tx.send(()).await;

        Ok(())
    }

    fn get_codec(&self) -> Arc<Mutex<Codec>> {
        self.codec.clone()
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

        let (tx, _rx) = mpsc::channel(1000);
        let (tx_2, mut rx_2) = mpsc::channel(1000);

        let (_mic_tx, mic_rx) = mpsc::channel(1000);

        let audio_handler_handle =
            tokio::spawn(async move { audio_handler.start(mic_rx, tx).await });

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

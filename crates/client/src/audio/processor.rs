use super::{codec::AudioCodec, SoundProcessor};
use crate::error::ClientError;
use common::packet::{AudioPacket, Packet};
use std::sync::Arc;
use std::u32;
use tokio::{
    join,
    sync::{
        broadcast,
        mpsc::{self},
        Mutex,
    },
};

pub struct AudioProcessor<Codec: AudioCodec> {
    codec: Arc<Mutex<Codec>>,

    stop_tx: Arc<Mutex<Option<broadcast::Sender<()>>>>,
}

impl<Codec: AudioCodec> AudioProcessor<Codec> {}

#[async_trait::async_trait]
impl<Codec: AudioCodec + 'static> SoundProcessor for AudioProcessor<Codec> {
    type Codec = Codec;

    fn new() -> Result<Self, ClientError> {
        Ok(AudioProcessor {
            codec: Arc::new(Mutex::new(Codec::new()?)),
            stop_tx: Arc::new(Mutex::new(None)),
        })
    }

    async fn start(
        &self,
        mut mic_rx: mpsc::Receiver<Vec<f32>>,
        packet_sender: mpsc::Sender<Packet>,
    ) {
        println!("Starting audio handler");

        let (audio_tx, mut audio_rx) = mpsc::channel::<Vec<u8>>(20);
        let codec = self.codec.clone();
        let (stop_tx, mut stop_rx) = broadcast::channel::<()>(1);
        let mut stop_rx_clone = stop_rx.resubscribe();
        {
            *self.stop_tx.lock().await = Some(stop_tx);
        }

        let microphone_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(audio_samples) = mic_rx.recv() => {
                        match codec.lock().await.encode(audio_samples){
                            Ok(encoded_data) => {
                                let _ = audio_tx.send(encoded_data).await;
                            },
                            Err(e) =>{
                                println!("Failed to encode audio samples {:?}", e);
                            }
                        };
                    }
                    _ = stop_rx.recv() => break
                }
            }
        });

        let audio_packets_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(track) = audio_rx.recv() => {
                        match Packet::new(AudioPacket { track }){
                            Ok(packet) => {
                                let _ = packet_sender.send(packet).await;
                            },
                            Err(e) => {
                                println!("Failed to create audio packet {:?}", e);
                            }
                        }
                    }
                    _ = stop_rx_clone.recv() => break
                }
            }
        });

        match join!(microphone_handle, audio_packets_handle) {
            _ => {}
        };
    }

    async fn stop(&self) {
        let mut stop_tx = self.stop_tx.lock().await;
        if stop_tx.is_none() {
            println!("Audio handler is not running");
            return;
        }

        let _ = stop_tx.take().unwrap().send(());
        println!("Stopped audio handler");
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

    #[tokio::test(flavor = "multi_thread")]
    async fn test_audio_handler() {
        let audio_handler = AudioProcessor::<OpusAudioCodec>::new().unwrap();
        {
            audio_handler
                .get_codec()
                .lock()
                .await
                .update(48000, 1)
                .unwrap();
        }

        let (mic_tx, mic_rx) = mpsc::channel::<Vec<f32>>(20);
        let (packet_tx, mut packet_rx) = mpsc::channel::<Packet>(20);

        tokio::spawn(async move {
            let _ = audio_handler.start(mic_rx, packet_tx).await;
        });

        let audio_samples = vec![10.0; 480];
        tokio::spawn(async move {
            for _ in 0..10 {
                let _ = mic_tx.send(audio_samples.clone()).await;
            }
        });

        for _ in 0..10 {
            let packet = packet_rx.recv().await.unwrap();
            assert_eq!(packet.packet_id, 2);
            assert!(packet.data.len() > 75);
            assert!(packet.data.len() < 125);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_audio_handler_stop() {
        let audio_handler = AudioProcessor::<OpusAudioCodec>::new().unwrap();
        {
            audio_handler
                .get_codec()
                .lock()
                .await
                .update(48000, 1)
                .unwrap();
        }

        let audio_handler = Arc::new(audio_handler);
        let audio_handler_clone = audio_handler.clone();

        let (mic_tx, mic_rx) = mpsc::channel::<Vec<f32>>(1);
        let (packet_tx, mut packet_rx) = mpsc::channel::<Packet>(1);

        tokio::spawn(async move {
            let _ = audio_handler.clone().start(mic_rx, packet_tx).await;
        });

        let audio_samples = vec![10.0; 480];
        tokio::spawn(async move {
            for _ in 0..10 {
                let _ = mic_tx.send(audio_samples.clone()).await;
                sleep(Duration::from_millis(10)).await;
                let _ = audio_handler_clone.stop().await;
            }
        });

        sleep(Duration::from_millis(10)).await;
        let _ = packet_rx.recv().await.unwrap();
        for _ in 0..9 {
            sleep(Duration::from_millis(10)).await;
            assert!(packet_rx.recv().await.is_none());
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_audio_handler_stereo() {
        let audio_handler = AudioProcessor::<OpusAudioCodec>::new().unwrap();
        {
            audio_handler
                .get_codec()
                .lock()
                .await
                .update(48000, 2)
                .unwrap();
        }

        let (mic_tx, mic_rx) = mpsc::channel::<Vec<f32>>(20);
        let (packet_tx, mut packet_rx) = mpsc::channel::<Packet>(20);

        tokio::spawn(async move {
            let _ = audio_handler.start(mic_rx, packet_tx).await;
        });

        let audio_samples = vec![10.0; 480 * 2];
        tokio::spawn(async move {
            for _ in 0..10 {
                let _ = mic_tx.send(audio_samples.clone()).await;
            }
        });

        for _ in 0..10 {
            let packet = packet_rx.recv().await.unwrap();
            assert_eq!(packet.packet_id, 2);
            assert!(packet.data.len() > 100);
            assert!(packet.data.len() < 225);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_audio_handler_invalid_packet() {
        let audio_handler = AudioProcessor::<OpusAudioCodec>::new().unwrap();
        {
            audio_handler
                .get_codec()
                .lock()
                .await
                .update(48000, 1)
                .unwrap();
        }

        let (mic_tx, mic_rx) = mpsc::channel::<Vec<f32>>(20);
        let (packet_tx, mut packet_rx) = mpsc::channel::<Packet>(20);

        tokio::spawn(async move {
            let _ = audio_handler.start(mic_rx, packet_tx).await;
        });

        let audio_samples = vec![10.0; 4890];
        tokio::spawn(async move {
            for _ in 0..3 {
                let _ = mic_tx.send(audio_samples.clone()).await;
            }
            let _ = mic_tx.send(vec![0.0; 480]).await;
        });

        for _ in 0..3 {
            assert!(packet_rx.try_recv().is_err());
        }

        sleep(Duration::from_millis(10)).await;
        let packet = packet_rx.try_recv().unwrap();
        assert_eq!(packet.packet_id, 2);
        assert!(packet.data.len() > 10);
        assert!(packet.data.len() < 15);

        for _ in 0..10 {
            assert!(packet_rx.try_recv().is_err());
        }
    }
}

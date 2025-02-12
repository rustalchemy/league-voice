use super::AudioCodec;
use crate::error::ClientError;
use opus::{Application, Decoder, Encoder};
use rubato::{FftFixedIn, Resampler};
use std::sync::Mutex;

const SAMPLE_RATE: u32 = 48000;
const CHANNELS: usize = 1;
const CHANNEL: opus::Channels = opus::Channels::Mono;
const FRAME_SIZE: usize = (SAMPLE_RATE as usize / 1000 * 20) * CHANNELS;

pub struct OpusAudioCodec {
    encoder: Mutex<Encoder>,
    decoder: Mutex<Decoder>,

    resampler: Mutex<Option<FftFixedIn<f32>>>,

    sample_rate: u32,
    channels: usize,
    chunk_size: usize,

    transform_buffer: Vec<Vec<f32>>,
}

impl AudioCodec for OpusAudioCodec {
    fn new() -> Result<Self, ClientError> {
        Ok(OpusAudioCodec {
            encoder: Mutex::new(Encoder::new(SAMPLE_RATE, CHANNEL, Application::Audio)?),
            decoder: Mutex::new(Decoder::new(SAMPLE_RATE, CHANNEL)?),

            resampler: Mutex::new(None),

            sample_rate: SAMPLE_RATE,
            channels: CHANNELS,
            chunk_size: SAMPLE_RATE as usize / 100 * CHANNELS,

            transform_buffer: vec![vec![0.0; FRAME_SIZE / CHANNELS]; CHANNELS],
        })
    }

    fn update(&mut self, sample_rate: u32, channels: usize) -> Result<(), ClientError> {
        self.channels = channels;
        self.sample_rate = sample_rate;
        self.chunk_size = sample_rate as usize / 100;
        self.transform_buffer = vec![vec![0.0; self.chunk_size]; self.channels];

        self.resampler
            .lock()
            .unwrap()
            .replace(FftFixedIn::<f32>::new(
                sample_rate as usize,
                SAMPLE_RATE as usize,
                self.chunk_size,
                1,
                self.channels,
            )?);

        Ok(())
    }

    fn encode(&mut self, data: Vec<f32>) -> Result<Vec<u8>, ClientError> {
        for (frame_idx, frame) in data.chunks(self.channels).enumerate() {
            for (channel_idx, &sample) in frame.iter().enumerate() {
                match self.transform_buffer[channel_idx].get_mut(frame_idx) {
                    None => {
                        return Err(ClientError::BufferOverflow);
                    }
                    Some(smpl) => *smpl = sample,
                }
            }
        }

        let resampled: Vec<f32> = {
            let mut resampler = self.resampler.lock().unwrap();
            let resampled = resampler
                .as_mut()
                .unwrap()
                .process(&self.transform_buffer, None)?;
            resampled.into_iter().flatten().collect()
        };

        let mut encoded = vec![0u8; FRAME_SIZE];
        let len = self
            .encoder
            .lock()
            .unwrap()
            .encode_float(&resampled, &mut encoded)?;
        encoded.truncate(len);

        Ok(encoded)
    }

    fn decode(&mut self, data: Vec<u8>) -> Result<Vec<f32>, ClientError> {
        let mut decoded = vec![0.0; FRAME_SIZE];
        let len = self
            .decoder
            .lock()
            .unwrap()
            .decode_float(&data, &mut decoded, false)?;
        decoded.truncate(len);
        Ok(decoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_create_new_opus_audio_codec() {
        assert!(OpusAudioCodec::new().is_ok());
    }

    #[test]
    fn should_encode_and_decode_audio_data() {
        let mut codec = OpusAudioCodec::new().unwrap();
        codec.update(48000, 1).unwrap();
        let data = vec![0.0; 480];
        let encoded = codec.encode(data.clone()).unwrap();
        let decoded = codec.decode(encoded).unwrap();
        for (a, b) in data.iter().zip(decoded.iter()) {
            assert!((a - b).abs() < 0.1e-10);
        }
    }

    #[test]
    fn should_encode_and_decode_audio_data_stereo() {
        let mut codec = OpusAudioCodec::new().unwrap();
        codec.update(48000, 2).unwrap();
        let data = vec![0.0; 480 * 2];
        let encoded = codec.encode(data.clone()).unwrap();
        let decoded = codec.decode(encoded).unwrap();
        for (a, b) in data.iter().zip(decoded.iter()) {
            assert!((a - b).abs() < 0.1e-2);
        }
    }

    #[test]
    fn should_encode_small_buffer() {
        let mut codec = OpusAudioCodec::new().unwrap();
        codec.update(48000, 1).unwrap();
        let data = vec![0.0; 1];
        assert!(codec.encode(data).is_ok());
    }

    #[test]
    fn should_fail_to_decode_audio_data() {
        let mut codec = OpusAudioCodec::new().unwrap();
        codec.update(48000, 1).unwrap();
        let data = vec![0; 2000];
        assert!(codec.decode(data).is_err());
    }

    #[test]
    fn should_fail_to_encode_audio_data() {
        let mut codec = OpusAudioCodec::new().unwrap();
        codec.update(48000, 1).unwrap();
        let data = vec![0.0; 2000];
        assert!(codec.encode(data).is_err());
    }

    #[test]
    fn should_fail_to_encode_audio_data_stereo() {
        let mut codec = OpusAudioCodec::new().unwrap();
        codec.update(48000, 2).unwrap();
        let data = vec![0.0; 2000];
        assert!(codec.encode(data).is_err());
    }
}

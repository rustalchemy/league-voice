use super::AudioCodec;
use crate::error::ClientError;
use opus::{Application, Decoder, Encoder};
use std::sync::Mutex;

#[derive(Debug)]
pub struct OpusAudioCodec {
    encoder: Mutex<Encoder>,
    decoder: Mutex<Decoder>,

    frame_size: usize,
}

impl AudioCodec for OpusAudioCodec {
    #[cfg(not(tarpaulin_include))]
    fn new(sample_rate: u32, channels: usize) -> Result<Self, ClientError> {
        let channel = match channels {
            1 => opus::Channels::Mono,
            2 => opus::Channels::Stereo,
            _ => return Err(ClientError::InvalidChannelCount),
        };

        let sample_rate = match sample_rate {
            8000 | 12000 | 16000 | 24000 | 48000 => sample_rate,
            _ => 48000,
        };

        println!(
            "Creating Opus codec with sample rate: {}, channels: {}",
            sample_rate, channels
        );

        let encoder = match Encoder::new(sample_rate, channel, Application::Audio) {
            Ok(encoder) => encoder,
            Err(err) => {
                eprintln!("Failed to create Opus encoder: {}", err);
                return Err(ClientError::OpusError(err));
            }
        };

        Ok(OpusAudioCodec {
            encoder: Mutex::new(encoder),
            decoder: Mutex::new(Decoder::new(sample_rate, channel)?),
            frame_size: (sample_rate as usize / 1000 * 20) * channels,
        })
    }

    fn encode(&self, data: Vec<f32>) -> Result<Vec<u8>, ClientError> {
        let mut encoded = vec![0; self.frame_size];
        let len = self
            .encoder
            .lock()
            .unwrap()
            .encode_float(&data, &mut encoded)?;
        encoded.truncate(len);
        Ok(encoded)
    }

    fn decode(&self, data: Vec<u8>) -> Result<Vec<f32>, ClientError> {
        let mut decoded = vec![0.0; self.frame_size];
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
        assert!(OpusAudioCodec::new(48000, 1).is_ok());
        assert!(OpusAudioCodec::new(48000, 2).is_ok());
        assert!(OpusAudioCodec::new(48000, 3).is_err());
    }

    #[test]
    fn should_encode_and_decode_audio_data() {
        let codec = OpusAudioCodec::new(48000, 1).unwrap();
        let data = vec![0.0; 960];
        let encoded = codec.encode(data.clone()).unwrap();
        let decoded = codec.decode(encoded).unwrap();
        for (a, b) in data.iter().zip(decoded.iter()) {
            let difference = (a - b).abs();
            assert!(difference < 0.1e-10, "Difference: {}", difference);
        }
    }

    #[test]
    fn should_encode_and_decode_audio_data_stereo() {
        let codec = OpusAudioCodec::new(48000, 2).unwrap();
        let data = vec![0.0; 960 * 2];
        let encoded = codec.encode(data.clone()).unwrap();
        let decoded = codec.decode(encoded).unwrap();
        for (a, b) in data.iter().zip(decoded.iter()) {
            let difference = (a - b).abs();
            assert!(difference < 0.1e-2, "Difference: {}", difference);
        }
    }

    #[test]
    fn should_fail_to_encode_audio_data() {
        let codec = OpusAudioCodec::new(48000, 1).unwrap();
        let data = vec![0.0; 1];
        assert!(codec.encode(data).is_err());
    }

    #[test]
    fn should_fail_to_decode_audio_data() {
        let codec = OpusAudioCodec::new(48000, 1).unwrap();
        let data = vec![0; 2000];
        assert!(codec.decode(data).is_err());
    }
}

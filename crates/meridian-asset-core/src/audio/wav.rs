//! Hand-rolled PCM WAV decoder — simple enough to own outright, unlike
//! the compressed formats (see `compressed_audio`).

use crate::{AudioData, DecodeError, Decoder, need, u16_le, u32_le};

/// Decodes PCM WAV (`fmt ` chunk with `audioFormat == 1`, 16-bit samples
/// only). Chunk-based, so it tolerates extra chunks before `data`.
#[derive(Debug, Default)]
pub struct WavDecoder;

impl Decoder<AudioData> for WavDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<AudioData, DecodeError> {
        need(bytes, 12)?;
        if &bytes[0..4] != b"RIFF" {
            return Err(DecodeError::BadMagic { expected: "RIFF" });
        }
        if &bytes[8..12] != b"WAVE" {
            return Err(DecodeError::BadMagic { expected: "WAVE" });
        }

        let mut sample_rate = None;
        let mut channels = None;
        let mut bits_per_sample = None;
        let mut data: Option<&[u8]> = None;

        let mut offset = 12usize;
        while offset + 8 <= bytes.len() {
            let chunk_id = &bytes[offset..offset + 4];
            let chunk_size = u32_le(bytes, offset + 4) as usize;
            let body_start = offset + 8;
            need(bytes, body_start + chunk_size)?;
            let body = &bytes[body_start..body_start + chunk_size];

            if chunk_id == b"fmt " {
                need(body, 16)?;
                let audio_format = u16_le(body, 0);
                if audio_format != 1 {
                    return Err(DecodeError::Unsupported("WAV audioFormat other than PCM"));
                }
                channels = Some(u16_le(body, 2));
                sample_rate = Some(u32_le(body, 4));
                bits_per_sample = Some(u16_le(body, 14));
            } else if chunk_id == b"data" {
                data = Some(body);
            }

            // Chunks are padded to even length.
            offset = body_start + chunk_size + (chunk_size % 2);
        }

        let sample_rate = sample_rate.ok_or(DecodeError::Malformed("missing fmt chunk"))?;
        let channels = channels.ok_or(DecodeError::Malformed("missing fmt chunk"))?;
        let bits_per_sample = bits_per_sample.ok_or(DecodeError::Malformed("missing fmt chunk"))?;
        let data = data.ok_or(DecodeError::Malformed("missing data chunk"))?;

        if bits_per_sample != 16 {
            return Err(DecodeError::Unsupported("WAV bit depth other than 16"));
        }
        if data.len() % 2 != 0 {
            return Err(DecodeError::Malformed(
                "data chunk length not a multiple of sample size",
            ));
        }

        let samples = data
            .chunks_exact(2)
            .map(|s| i16::from_le_bytes([s[0], s[1]]))
            .collect();

        Ok(AudioData {
            sample_rate,
            channels,
            samples,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_wav(sample_rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let data_bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let fmt_body_len = 16u32;
        let data_len = data_bytes.len() as u32;
        let riff_len = 4 + (8 + fmt_body_len) + (8 + data_len);

        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&riff_len.to_le_bytes());
        b.extend_from_slice(b"WAVE");

        b.extend_from_slice(b"fmt ");
        b.extend_from_slice(&fmt_body_len.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes()); // PCM
        b.extend_from_slice(&channels.to_le_bytes());
        b.extend_from_slice(&sample_rate.to_le_bytes());
        let block_align = channels * 2;
        let byte_rate = sample_rate * block_align as u32;
        b.extend_from_slice(&byte_rate.to_le_bytes());
        b.extend_from_slice(&block_align.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

        b.extend_from_slice(b"data");
        b.extend_from_slice(&data_len.to_le_bytes());
        b.extend_from_slice(&data_bytes);
        b
    }

    #[test]
    fn wav_decodes_pcm_samples_and_format() {
        let samples = [0i16, 1000, -1000, i16::MAX, i16::MIN];
        let bytes = make_wav(44100, 2, &samples);
        let audio = WavDecoder.decode(&bytes).unwrap();
        assert_eq!(audio.sample_rate, 44100);
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.samples, samples);
    }

    #[test]
    fn wav_rejects_bad_magic() {
        let mut bytes = make_wav(44100, 1, &[0]);
        bytes[0] = b'X';
        assert_eq!(
            WavDecoder.decode(&bytes),
            Err(DecodeError::BadMagic { expected: "RIFF" })
        );
    }

    #[test]
    fn wav_tolerates_extra_chunks_before_data() {
        let mut bytes = make_wav(8000, 1, &[42]);
        // Splice a fake "LIST" chunk (4 bytes of body) right after "fmt ".
        let fmt_end = 12 + 8 + 16;
        let mut extra = Vec::new();
        extra.extend_from_slice(b"LIST");
        extra.extend_from_slice(&4u32.to_le_bytes());
        extra.extend_from_slice(&[1, 2, 3, 4]);
        bytes.splice(fmt_end..fmt_end, extra.iter().copied());
        // Fix up the RIFF size to account for the inserted chunk.
        let added = extra.len() as u32;
        let old_riff_len = u32_le(&bytes, 4);
        bytes[4..8].copy_from_slice(&(old_riff_len + added).to_le_bytes());

        let audio = WavDecoder.decode(&bytes).unwrap();
        assert_eq!(audio.samples, vec![42]);
    }
}

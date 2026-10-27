//! Audio container formats recognised by their magic bytes.

/// An audio container/codec identified from leading magic bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    /// RIFF/WAVE (`RIFF....WAVE`) — decoded by the hand-rolled
    /// [`WavDecoder`](super::WavDecoder).
    Wav,
    /// MPEG Layer III: an `ID3` tag or an MPEG frame-sync header.
    Mp3,
    /// OGG container whose first packet is `\x01vorbis`.
    OggVorbis,
    /// OGG container whose first packet is `OpusHead`.
    OggOpus,
    /// Native FLAC (`fLaC`).
    Flac,
}

impl AudioFormat {
    /// Identifies the format from the buffer's leading bytes. Returns
    /// `None` when no known signature matches — extension-based guessing
    /// is deliberately not a fallback.
    pub fn detect(bytes: &[u8]) -> Option<AudioFormat> {
        if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
            return Some(AudioFormat::Wav);
        }
        if bytes.len() >= 4 && &bytes[0..4] == b"fLaC" {
            return Some(AudioFormat::Flac);
        }
        if bytes.len() >= 4 && &bytes[0..4] == b"OggS" {
            // The first packet's payload starts right after the 27-byte
            // page header plus the segment table; its first bytes name
            // the codec ("OpusHead" / "\x01vorbis").
            let payload = bytes
                .get(26)
                .map(|&segments| 27 + segments as usize)
                .and_then(|start| bytes.get(start..));
            if let Some(payload) = payload {
                if payload.starts_with(b"OpusHead") {
                    return Some(AudioFormat::OggOpus);
                }
                if payload.starts_with(b"\x01vorbis") {
                    return Some(AudioFormat::OggVorbis);
                }
            }
            return None;
        }
        if bytes.len() >= 3 && &bytes[0..3] == b"ID3" {
            return Some(AudioFormat::Mp3);
        }
        // Raw MPEG audio frame sync: 11 set bits, then a valid
        // version/layer combination (layer bits `01` = Layer III).
        if bytes.len() >= 2
            && bytes[0] == 0xFF
            && bytes[1] & 0xE0 == 0xE0
            && bytes[1] & 0x06 == 0x02
        {
            return Some(AudioFormat::Mp3);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal single-page OGG stream: real page header (magic,
    /// version, flags, granule, serial, sequence, CRC left zero — enough
    /// for signature detection, not for demuxing), one segment carrying
    /// `payload`.
    fn make_ogg_page(payload: &[u8]) -> Vec<u8> {
        assert!(payload.len() < 255);
        let mut b = Vec::new();
        b.extend_from_slice(b"OggS");
        b.extend_from_slice(&[0u8; 22]); // version..CRC
        b.push(1); // one segment
        b.push(u8::try_from(payload.len()).unwrap()); // segment table
        b.extend_from_slice(payload);
        b
    }

    fn make_wav_header() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&36u32.to_le_bytes());
        b.extend_from_slice(b"WAVE");
        b
    }

    #[test]
    fn detect_identifies_every_signature() {
        assert_eq!(
            AudioFormat::detect(&make_wav_header()),
            Some(AudioFormat::Wav)
        );
        assert_eq!(AudioFormat::detect(b"fLaC\x00"), Some(AudioFormat::Flac));
        assert_eq!(
            AudioFormat::detect(b"ID3\x04\x00rest"),
            Some(AudioFormat::Mp3)
        );
        assert_eq!(
            AudioFormat::detect(&[0xFF, 0xFB, 0x90, 0x00]), // MPEG-1 Layer III sync
            Some(AudioFormat::Mp3)
        );
        assert_eq!(
            AudioFormat::detect(&make_ogg_page(b"OpusHead\x01\x02")),
            Some(AudioFormat::OggOpus)
        );
        assert_eq!(
            AudioFormat::detect(&make_ogg_page(b"\x01vorbis\x00")),
            Some(AudioFormat::OggVorbis)
        );
        assert_eq!(AudioFormat::detect(&make_ogg_page(b"\x7fFLAC")), None);
        assert_eq!(AudioFormat::detect(b"not audio at all"), None);
        assert_eq!(AudioFormat::detect(&[]), None);
        // Frame-sync-like byte pair with a reserved layer is not MP3.
        assert_eq!(AudioFormat::detect(&[0xFF, 0xE0]), None);
    }
}

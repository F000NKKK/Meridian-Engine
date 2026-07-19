//! Low-level audio device abstraction: output streams, buffers and platform audio backends.

/// An OS audio output device.
#[derive(Debug)]
pub struct AudioDevice;

/// An open audio output stream.
#[derive(Debug, Clone, Copy)]
pub struct AudioStream {
    pub sample_rate: u32,
    pub channels: u16,
}

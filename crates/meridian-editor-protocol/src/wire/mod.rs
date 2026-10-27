//! Messages on a byte stream: Substrate Data frames, a little-endian `u32`
//! length and then the message.

mod frame_error;
mod read_message;
mod write_message;

pub use frame_error::FrameError;
pub use read_message::read_message;
pub use write_message::write_message;

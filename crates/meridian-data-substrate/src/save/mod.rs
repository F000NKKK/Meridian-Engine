//! The `.msave` envelope: `{ format, game, created, scene, state }`, binary
//! only. The engine owns the envelope and the scene document inside it;
//! the game owns `state`.

mod envelope_reader;
mod envelope_writer;
mod error;
mod header;
mod loaded_save;

pub(crate) use envelope_reader::{read_header_and_scene, read_state};
pub(crate) use envelope_writer::write_envelope;
pub use error::SaveError;
pub use header::SaveHeader;
pub use loaded_save::LoadedSave;

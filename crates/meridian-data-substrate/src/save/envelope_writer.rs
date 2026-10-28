//! Writing the `.msave` envelope.

use substrate_data::binary::Writer;
use substrate_data::{Schema, Value};

use super::SaveHeader;
use crate::SAVE_FORMAT;
use crate::document_keys::{CREATED, FORMAT_KEY, GAME, SCENE, STATE};

/// The envelope in the binary form: the header's fields, the scene
/// document, then the game's `state` written by its own schema, straight —
/// so a [`substrate_data::Stream`] in it stays a stream. The fields are in
/// this order, which a reader can rely on to stop early; the format
/// written is always [`SAVE_FORMAT`].
pub(crate) fn write_envelope<T: Schema>(header: &SaveHeader, scene: &Value, state: &T) -> Vec<u8> {
    let mut w = Writer::new();
    w.map(5);
    w.key(FORMAT_KEY).int(i64::from(SAVE_FORMAT));
    w.key(GAME).str(&header.game);
    w.key(CREATED).int(header.created);
    w.key(SCENE).value(scene);
    w.key(STATE);
    state.encode(&mut w);
    w.finish()
}

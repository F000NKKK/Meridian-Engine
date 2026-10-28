//! Reading the `.msave` envelope: the fast way, step by step through the
//! binary form so only what is asked for is decoded, and, when that finds
//! anything irregular, the slow way, which says exactly what is wrong.

use substrate_data::binary::Decoder;
use substrate_data::{Bump, MapRef, Path, Schema, Value, binary, read_ref_value};

use super::{SaveError, SaveHeader};
use crate::SAVE_FORMAT;
use crate::document_keys::{CREATED, FORMAT_KEY, GAME, SCENE, STATE};

/// Whether a walk over the envelope's fields goes on.
enum Step {
    Continue,
    Stop,
}

/// The envelope's fields found so far.
#[derive(Default)]
struct Found {
    format: Option<u32>,
    game: Option<String>,
    created: Option<i64>,
    scene: Option<Value>,
}

/// The header of a save, and its scene document when `scene` is asked for.
/// Nothing after the last field asked for is looked at.
pub(crate) fn read_header_and_scene(
    bytes: &[u8],
    scene: bool,
) -> Result<(SaveHeader, Option<Value>), SaveError> {
    let found = match fast_header(bytes, scene) {
        Some(found) => found,
        None => slow_header(bytes)?,
    };
    supported(found.format)?;
    let missing = |field| SaveError::MissingField { field };
    let header = SaveHeader {
        format: SAVE_FORMAT,
        game: found.game.ok_or(missing(GAME))?,
        created: found.created.ok_or(missing(CREATED))?,
    };
    if scene && found.scene.is_none() {
        return Err(missing(SCENE));
    }
    Ok((header, found.scene.filter(|_| scene)))
}

/// The game's `state`, read by its schema. The whole envelope is walked and
/// checked, but only `state` is decoded.
pub(crate) fn read_state<T: Schema>(bytes: &[u8]) -> Result<T, SaveError> {
    let (mut format, mut state) = (None, None);
    let walked = walk(bytes, |key, d| {
        match key {
            FORMAT_KEY => format = Some(u32::decode(d)?),
            STATE => state = Some(T::decode(d)?),
            _ => d.skip()?,
        }
        Some(if format.is_some_and(|f| f != SAVE_FORMAT) {
            Step::Stop
        } else {
            Step::Continue
        })
    });
    if walked.is_some() {
        supported(format)?;
        if let Some(state) = state {
            return Ok(state);
        }
    }
    slow_state(bytes)
}

fn fast_header(bytes: &[u8], scene: bool) -> Option<Found> {
    let mut found = Found::default();
    walk(bytes, |key, d| {
        match key {
            FORMAT_KEY => found.format = Some(u32::decode(d)?),
            GAME => found.game = Some(String::decode(d)?),
            CREATED => found.created = Some(i64::decode(d)?),
            SCENE if scene => found.scene = Some(d.value().ok()?),
            _ => d.skip()?,
        }
        let wrong_format = found.format.is_some_and(|f| f != SAVE_FORMAT);
        let done = found.game.is_some() && found.created.is_some();
        Some(
            if wrong_format || (done && (!scene || found.scene.is_some())) {
                Step::Stop
            } else {
                Step::Continue
            },
        )
    })?;
    Some(found)
}

/// Goes through the envelope's fields in file order, handing each to
/// `each` with the decoder at its value. `None` for anything irregular
/// (the slow way then says what); a [`Step::Stop`] ends the walk without
/// looking at the rest.
fn walk<'b>(
    bytes: &'b [u8],
    mut each: impl FnMut(&'b str, &mut Decoder<'b>) -> Option<Step>,
) -> Option<()> {
    let mut d = Decoder::open(bytes).ok()?;
    for _ in 0..d.map()? {
        let key = d.key()?;
        if matches!(each(key, &mut d)?, Step::Stop) {
            return Some(());
        }
    }
    d.leave();
    d.at_end().then_some(())
}

fn slow_header(bytes: &[u8]) -> Result<Found, SaveError> {
    let arena = Bump::new();
    let root = binary::decode_ref(bytes, &arena).map_err(malformed)?;
    let map = root.as_map().ok_or(SaveError::NotAnEnvelope)?;
    let format = int_field(map, FORMAT_KEY, "an unsigned 32-bit number")?;
    let format = u32::try_from(format).map_err(|_| bad(FORMAT_KEY, "an unsigned 32-bit number"))?;
    supported(Some(format))?;
    let game = map
        .get(GAME)
        .ok_or(SaveError::MissingField { field: GAME })?;
    let game = game.as_str().ok_or(bad(GAME, "text"))?;
    Ok(Found {
        format: Some(format),
        game: Some(game.to_owned()),
        created: Some(int_field(map, CREATED, "a number of seconds")?),
        scene: map.get(SCENE).map(|s| s.to_value()),
    })
}

fn slow_state<T: Schema>(bytes: &[u8]) -> Result<T, SaveError> {
    let arena = Bump::new();
    let root = binary::decode_ref(bytes, &arena).map_err(malformed)?;
    let map = root.as_map().ok_or(SaveError::NotAnEnvelope)?;
    let format = int_field(map, FORMAT_KEY, "an unsigned 32-bit number")?;
    supported(u32::try_from(format).ok())?;
    let state = map
        .get(STATE)
        .ok_or(SaveError::MissingField { field: STATE })?;
    read_ref_value(state).map_err(|problems| {
        let under_state = |mut d: substrate_data::Diagnostic| {
            d.path = Path::root().key(STATE).then(&d.path);
            d
        };
        SaveError::State(problems.into_iter().map(under_state).collect())
    })
}

fn int_field(
    map: MapRef<'_>,
    field: &'static str,
    expected: &'static str,
) -> Result<i64, SaveError> {
    let value = map.get(field).ok_or(SaveError::MissingField { field })?;
    value.as_i64().ok_or(bad(field, expected))
}

fn bad(field: &'static str, expected: &'static str) -> SaveError {
    SaveError::BadField { field, expected }
}

fn malformed(e: binary::DecodeError) -> SaveError {
    SaveError::Malformed {
        message: e.message,
        offset: e.offset,
    }
}

/// `format` is the one this reads (a missing one is an error too).
fn supported(format: Option<u32>) -> Result<(), SaveError> {
    match format {
        Some(SAVE_FORMAT) => Ok(()),
        Some(found) => Err(SaveError::UnsupportedFormat {
            found,
            supported: SAVE_FORMAT,
        }),
        None => Err(SaveError::MissingField { field: FORMAT_KEY }),
    }
}

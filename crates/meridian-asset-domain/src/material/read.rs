//! Reading a `.mmat` file in either form.

use substrate_data::{Errors, Value, binary};

use super::document::MaterialDocument;
use super::{MATERIAL_FORMAT, MaterialAsset, MaterialError};

/// The magic bytes of the binary form.
const MAGIC: &[u8] = binary::MAGIC;

pub(super) fn read(bytes: &[u8]) -> Result<MaterialAsset, MaterialError> {
    let document = if bytes.starts_with(MAGIC) {
        read_binary(bytes)?
    } else {
        read_text(bytes)?
    };
    Ok(document.into_asset())
}

fn read_binary(bytes: &[u8]) -> Result<MaterialDocument, MaterialError> {
    let value = binary::decode(bytes).map_err(|e| MaterialError::Malformed {
        message: e.message,
        offset: e.offset,
    })?;
    check_format(&value)?;
    substrate_data::from_value::<MaterialDocument>(&value)
        .map(|read| read.value)
        .map_err(MaterialError::Invalid)
}

fn read_text(bytes: &[u8]) -> Result<MaterialDocument, MaterialError> {
    let text = std::str::from_utf8(bytes).map_err(|e| MaterialError::NotUtf8 {
        valid_up_to: e.valid_up_to(),
    })?;
    let Errors(problems) = match substrate_data::from_text::<Value>(text) {
        Ok(probe) => {
            check_format(&probe.value)?;
            match substrate_data::from_text::<MaterialDocument>(text) {
                Ok(read) => return Ok(read.value),
                Err(errors) => errors,
            }
        }
        Err(errors) => errors,
    };
    Err(MaterialError::Invalid(problems))
}

/// The `format` of the document, before its other fields are looked at, so a
/// newer file is "unsupported" and not a list of unknown fields.
fn check_format(value: &Value) -> Result<(), MaterialError> {
    let map = value.as_map().ok_or(MaterialError::NotAMap)?;
    let format = map
        .get("format")
        .and_then(Value::as_i64)
        .and_then(|n| u32::try_from(n).ok())
        .filter(|&n| n >= 1)
        .ok_or(MaterialError::BadFormat)?;
    if format > MATERIAL_FORMAT {
        return Err(MaterialError::UnsupportedFormat { found: format });
    }
    Ok(())
}

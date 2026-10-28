//! Reading a `.meta` file in either form.

use substrate_data::{Errors, Value, binary};

use super::document::MetaDocument;
use super::{AssetMeta, META_FORMAT, MetaError};

/// The magic bytes of the binary form.
const MAGIC: &[u8] = binary::MAGIC;

pub(super) fn read(bytes: &[u8]) -> Result<AssetMeta, MetaError> {
    let document = if bytes.starts_with(MAGIC) {
        read_binary(bytes)?
    } else {
        read_text(bytes)?
    };
    Ok(AssetMeta {
        settings: document.settings()?,
        dependencies: document.dependencies()?,
        id: document.id,
        importer: document.importer,
    })
}

fn read_binary(bytes: &[u8]) -> Result<MetaDocument, MetaError> {
    let value = binary::decode(bytes).map_err(|e| MetaError::Malformed {
        message: e.message,
        offset: e.offset,
    })?;
    check_format(&value)?;
    substrate_data::from_value::<MetaDocument>(&value)
        .map(|read| read.value)
        .map_err(MetaError::Invalid)
}

fn read_text(bytes: &[u8]) -> Result<MetaDocument, MetaError> {
    let text = std::str::from_utf8(bytes).map_err(|e| MetaError::NotUtf8 {
        valid_up_to: e.valid_up_to(),
    })?;
    let Errors(problems) = match substrate_data::from_text::<Value>(text) {
        Ok(probe) => {
            check_format(&probe.value)?;
            match substrate_data::from_text::<MetaDocument>(text) {
                Ok(read) => return Ok(read.value),
                Err(errors) => errors,
            }
        }
        Err(errors) => errors,
    };
    Err(MetaError::Invalid(problems))
}

/// The `format` of the document, before its other fields are looked at, so a
/// newer file is "unsupported" and not a list of unknown fields.
fn check_format(value: &Value) -> Result<(), MetaError> {
    let map = value.as_map().ok_or(MetaError::NotAMap)?;
    let format = map
        .get("format")
        .and_then(Value::as_i64)
        .and_then(|n| u32::try_from(n).ok())
        .filter(|&n| n >= 1)
        .ok_or(MetaError::BadFormat)?;
    if format > META_FORMAT {
        return Err(MetaError::UnsupportedFormat { found: format });
    }
    Ok(())
}

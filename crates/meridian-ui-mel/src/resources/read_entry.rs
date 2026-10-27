//! Reading one `name: value` entry of a `.mres` document.

use substrate_data::typed::{self, Spelled};
use substrate_data::{Map, Value};

use super::resource_set::constant_name;
use super::{ResourceEntry, ResourceProblem, ResourceValue};

/// Reads the entry `name: value`.
pub(super) fn read_entry(name: &str, value: &Value) -> Result<ResourceEntry, ResourceProblem> {
    if !is_resource_name(name) {
        return Err(ResourceProblem::InvalidName(name.to_string()));
    }
    let (value, comment) = match value {
        Value::Map(fields) => read_object(name, fields)?,
        bare => (read_value(name, bare_type(bare), bare)?, None),
    };
    Ok(ResourceEntry {
        constant: constant_name(name),
        name: name.to_string(),
        value,
        comment,
    })
}

/// A bare value's type: numbers and flags are themselves, anything else
/// is text.
fn bare_type(value: &Value) -> &'static str {
    match value {
        Value::Int(_) | Value::Float(_) => "number",
        Value::Bool(_) => "bool",
        _ => "string",
    }
}

fn read_object(
    name: &str,
    fields: &Map,
) -> Result<(ResourceValue, Option<String>), ResourceProblem> {
    if let Some(field) = fields
        .keys()
        .find(|key| !["value", "type", "comment"].contains(key))
    {
        return Err(ResourceProblem::UnknownField {
            entry: name.to_string(),
            field: field.to_string(),
        });
    }
    let type_name = match fields.get("type") {
        None => "string",
        Some(Value::Str(found)) => found.as_str(),
        Some(_) => return Err(wrong(name, "a type name")),
    };
    let value = fields
        .get("value")
        .ok_or_else(|| ResourceProblem::MissingValue(name.to_string()))?;
    let comment = fields
        .get("comment")
        .and_then(Value::as_str)
        .map(str::to_string);
    Ok((read_value(name, type_name, value)?, comment))
}

fn read_value(
    name: &str,
    type_name: &str,
    value: &Value,
) -> Result<ResourceValue, ResourceProblem> {
    match (type_name, value) {
        ("string", Value::Str(raw)) => read_text(name, raw),
        ("number", Value::Int(number)) => Ok(ResourceValue::Integer(*number)),
        ("number", Value::Float(number)) => Ok(ResourceValue::Float(*number)),
        ("bool", Value::Bool(flag)) => Ok(ResourceValue::Bool(*flag)),
        ("color", Value::Str(raw)) => read_color(name, raw),
        ("string", _) => Err(wrong(name, "text")),
        ("number", _) => Err(wrong(name, "a number")),
        ("bool", _) => Err(wrong(name, "true or false")),
        ("color", _) => Err(wrong(name, "a color string")),
        _ => Err(ResourceProblem::UnknownType {
            entry: name.to_string(),
            found: type_name.to_string(),
        }),
    }
}

fn wrong(name: &str, expected: &'static str) -> ResourceProblem {
    ResourceProblem::WrongValue {
        entry: name.to_string(),
        expected,
    }
}

/// A bare string is a number or flag only through its type; here it is
/// text, so a reference or bytes is a mistake.
pub(super) fn read_text(name: &str, raw: &str) -> Result<ResourceValue, ResourceProblem> {
    match typed::classify(raw) {
        Ok(Spelled::Plain(text)) => Ok(ResourceValue::Text(text)),
        Ok(other) => Err(ResourceProblem::NotPlainText {
            entry: name.to_string(),
            message: format!(
                "{} cannot be a text; write '@@' for a leading '@'",
                other.kind()
            ),
        }),
        Err(error) => Err(ResourceProblem::NotPlainText {
            entry: name.to_string(),
            message: error.to_string(),
        }),
    }
}

pub(super) fn read_color(name: &str, raw: &str) -> Result<ResourceValue, ResourceProblem> {
    let bad = || ResourceProblem::BadColor {
        entry: name.to_string(),
        found: raw.to_string(),
    };
    let digits = raw.strip_prefix('#').ok_or_else(bad)?;
    let argb = u32::from_str_radix(digits, 16).map_err(|_| bad())?;
    match digits.len() {
        6 => Ok(ResourceValue::Color(0xFF00_0000 | argb)),
        8 => Ok(ResourceValue::Color(argb)),
        _ => Err(bad()),
    }
}

/// Dotted identifiers: `title`, `screen.title`.
fn is_resource_name(name: &str) -> bool {
    !name.is_empty()
        && name.split('.').all(|part| {
            part.starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_')
                && part
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        })
}

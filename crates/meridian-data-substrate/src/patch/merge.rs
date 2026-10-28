//! [`merged`]: a patch applied to a document value.

use substrate_data::Value;

/// `patch` applied to `base` (`None` when there is nothing there yet): maps
/// merge key by key, recursively, and anything else (a scalar, a list, a
/// `null`, a map over a value that is not a map) replaces what was there.
/// A `null` in a patch is a value like any other, not a deletion.
pub(crate) fn merged(base: Option<Value>, patch: &Value) -> Value {
    let (Some(Value::Map(mut base)), Value::Map(patch)) = (base, patch) else {
        return patch.clone();
    };
    for (key, field) in patch.iter() {
        match base.get_mut(key) {
            Some(slot) => *slot = merged(Some(std::mem::take(slot)), field),
            None => {
                base.insert(key, field.clone());
            }
        }
    }
    Value::Map(base)
}

#[cfg(test)]
mod tests {
    use substrate_data::Map;

    use super::*;

    fn map(fields: &[(&str, Value)]) -> Value {
        let mut m = Map::new();
        for (k, v) in fields {
            m.insert(*k, v.clone());
        }
        Value::Map(m)
    }

    #[test]
    fn maps_merge_recursively_and_the_rest_replaces() {
        let base = map(&[
            ("mass", Value::Int(80)),
            ("box", map(&[("w", Value::Int(1)), ("h", Value::Int(2))])),
            ("tags", Value::List(vec![Value::Int(1), Value::Int(2)])),
        ]);
        let patch = map(&[
            ("box", map(&[("h", Value::Int(5))])),
            ("tags", Value::List(vec![Value::Int(9)])),
            ("extra", Value::Null),
        ]);
        let want = map(&[
            ("mass", Value::Int(80)),
            ("box", map(&[("w", Value::Int(1)), ("h", Value::Int(5))])),
            ("tags", Value::List(vec![Value::Int(9)])),
            ("extra", Value::Null),
        ]);
        assert_eq!(merged(Some(base), &patch), want);
    }

    #[test]
    fn a_patch_over_nothing_or_a_non_map_is_the_patch() {
        let patch = map(&[("a", Value::Int(1))]);
        assert_eq!(merged(None, &patch), patch);
        assert_eq!(merged(Some(Value::Int(3)), &patch), patch);
        assert_eq!(merged(Some(patch.clone()), &Value::Int(3)), Value::Int(3));
    }
}

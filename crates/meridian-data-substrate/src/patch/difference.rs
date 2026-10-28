//! [`patch_between`]: the smallest patch that turns one value into another.

use substrate_data::{Map, Value};

/// The patch that [`super::merged`] turns `base` into `current` with, holding
/// only what changed; `None` when no patch can (a map lost a key, and a
/// patch has no way to delete one). `base` and `current` must differ.
pub(crate) fn patch_between(base: &Value, current: &Value) -> Option<Value> {
    let (Value::Map(base), Value::Map(current)) = (base, current) else {
        return Some(current.clone());
    };
    if base.keys().any(|key| current.get(key).is_none()) {
        return None;
    }
    let mut patch = Map::new();
    for (key, now) in current.iter() {
        match base.get(key) {
            Some(was) if was == now => {}
            Some(was) => {
                patch.insert(key, patch_between(was, now)?);
            }
            None => {
                patch.insert(key, now.clone());
            }
        }
    }
    Some(Value::Map(patch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::merged;

    fn map(fields: &[(&str, Value)]) -> Value {
        let mut m = Map::new();
        for (k, v) in fields {
            m.insert(*k, v.clone());
        }
        Value::Map(m)
    }

    #[test]
    fn only_the_changed_fields_are_in_the_patch() {
        let base = map(&[
            ("mass", Value::Int(80)),
            ("box", map(&[("w", Value::Int(1)), ("h", Value::Int(2))])),
        ]);
        let now = map(&[
            ("mass", Value::Int(80)),
            ("box", map(&[("w", Value::Int(1)), ("h", Value::Int(5))])),
        ]);
        let patch = patch_between(&base, &now).unwrap();
        assert_eq!(patch, map(&[("box", map(&[("h", Value::Int(5))]))]));
        assert_eq!(merged(Some(base), &patch), now);
    }

    #[test]
    fn a_lost_key_has_no_patch() {
        let base = map(&[("a", Value::Int(1)), ("b", Value::Int(2))]);
        let now = map(&[("a", Value::Int(1))]);
        assert_eq!(patch_between(&base, &now), None);
        let nested = |m: &Value| map(&[("in", m.clone())]);
        assert_eq!(patch_between(&nested(&base), &nested(&now)), None);
    }

    #[test]
    fn lists_scalars_and_new_keys_are_written_whole() {
        let base = map(&[
            ("tags", Value::List(vec![Value::Int(1)])),
            ("n", Value::Int(1)),
        ]);
        let now = map(&[
            ("tags", Value::List(vec![Value::Int(1), Value::Int(2)])),
            ("n", Value::Str("one".into())),
            ("fresh", map(&[("x", Value::Bool(true))])),
        ]);
        let patch = patch_between(&base, &now).unwrap();
        assert_eq!(merged(Some(base), &patch), now);
        assert_eq!(
            patch_between(&Value::Int(1), &Value::Int(2)),
            Some(Value::Int(2))
        );
    }
}

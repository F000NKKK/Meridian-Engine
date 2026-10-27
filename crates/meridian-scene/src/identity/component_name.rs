//! [`ComponentName`]: what a document calls a registered component.

use std::fmt;
use std::str::FromStr;

use substrate_data::KindName;
use substrate_data::schema::NamedType;

/// The largest edit distance a typo is suggested for, per three letters.
const TYPO_LETTERS: usize = 3;

/// The name a document gives a component type (`transform`, `rigid_body`):
/// lower case letters, digits and `_`, starting with a letter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ComponentName(Box<str>);

impl ComponentName {
    /// `name` as a component name.
    ///
    /// # Errors
    ///
    /// [`ComponentNameError`] when `name` isn't lower case letters, digits
    /// and `_` starting with a letter.
    pub fn new(name: &str) -> Result<Self, ComponentNameError> {
        if KindName::is_valid(name) {
            Ok(ComponentName(name.into()))
        } else {
            Err(ComponentNameError {
                text: name.to_owned(),
            })
        }
    }

    /// As written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The one of `known` closest to `self`, when close enough to be a typo.
    pub(crate) fn closest<'k>(
        &self,
        known: impl Iterator<Item = &'k ComponentName>,
    ) -> Option<&'k ComponentName> {
        let limit = self.0.len().div_ceil(TYPO_LETTERS).max(1);
        known
            .map(|k| (edit_distance(&self.0, &k.0), k))
            .filter(|(d, _)| *d <= limit)
            .min_by_key(|(d, _)| *d)
            .map(|(_, k)| k)
    }
}

impl fmt::Display for ComponentName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ComponentName {
    type Err = ComponentNameError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ComponentName::new(s)
    }
}

impl NamedType for ComponentName {
    const NAME: &'static str = "component_name";
    type Stored = String;
    fn to_stored(&self) -> String {
        self.0.to_string()
    }
    fn from_stored(stored: String) -> Option<Self> {
        ComponentName::new(&stored).ok()
    }
}

substrate_data::named_schema!(ComponentName);

/// A text that isn't a component name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentNameError {
    /// What was given.
    pub text: String,
}

impl fmt::Display for ComponentNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` isn't a component name (lower case letters, digits and `_`)",
            self.text
        )
    }
}

impl std::error::Error for ComponentNameError {}

/// Levenshtein distance of two names.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let swap = prev[j] + usize::from(ca != *cb);
            cur[j + 1] = swap.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(s: &str) -> ComponentName {
        ComponentName::new(s).unwrap()
    }

    #[test]
    fn names_are_checked_once() {
        assert!(ComponentName::new("rigid_body").is_ok());
        for bad in ["", "Rigid", "rigid body", "1st", "@x"] {
            assert_eq!(
                ComponentName::new(bad),
                Err(ComponentNameError { text: bad.into() })
            );
        }
    }

    #[test]
    fn a_typo_suggests_the_closest_name() {
        let known = [name("transform"), name("rigid_body"), name("follow")];
        assert_eq!(name("trasform").closest(known.iter()), Some(&known[0]));
        assert_eq!(name("rigidbody").closest(known.iter()), Some(&known[1]));
        assert_eq!(name("light").closest(known.iter()), None);
    }
}

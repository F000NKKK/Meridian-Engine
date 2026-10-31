//! Element and attribute names as XAML spells them: an optional `x:`
//! directive prefix, an optional owner type (`Grid.Row`,
//! `Entity.Components`) and a local name.

use std::fmt;

use super::prefix::Prefix;

/// A parsed name: `[x:][Owner.]local`. Owner and local are identifiers
/// (`[A-Za-z_][A-Za-z0-9_-]*`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QualifiedName {
    prefix: Option<Prefix>,
    owner: Option<String>,
    local: String,
}

/// Why a name was rejected by [`QualifiedName::parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualifiedNameError {
    /// Empty, or not an identifier where one was expected.
    Malformed,
    /// A prefix other than `x:`.
    UnknownPrefix(String),
}

impl QualifiedName {
    /// A plain name with no prefix and no owner.
    pub fn plain(local: impl Into<String>) -> Self {
        Self {
            prefix: None,
            owner: None,
            local: local.into(),
        }
    }

    /// Parses the full text of a name, e.g. `x:Name` or `Grid.Row`.
    pub fn parse(text: &str) -> Result<Self, QualifiedNameError> {
        let (prefix, rest) = match text.split_once(':') {
            Some(("x", rest)) => (Some(Prefix::Directive), rest),
            Some((other, _)) => return Err(QualifiedNameError::UnknownPrefix(other.to_string())),
            None => (None, text),
        };
        let (owner, local) = match rest.split_once('.') {
            Some((owner, local)) => (Some(owner), local),
            None => (None, rest),
        };
        if !owner.is_none_or(is_identifier) || !is_identifier(local) {
            return Err(QualifiedNameError::Malformed);
        }
        Ok(Self {
            prefix,
            owner: owner.map(str::to_string),
            local: local.to_string(),
        })
    }

    /// Returns the namespace directive prefix, if one was present.
    pub fn prefix(&self) -> Option<Prefix> {
        self.prefix
    }

    /// The owner type of a property element or attached property:
    /// `Grid` in `Grid.Row`.
    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }

    /// Returns the local name without its prefix or owner.
    pub fn local(&self) -> &str {
        &self.local
    }

    /// Whether this is the directive `x:local` (no owner).
    pub fn is_directive(&self, local: &str) -> bool {
        self.prefix == Some(Prefix::Directive) && self.owner.is_none() && self.local == local
    }

    /// Whether this is exactly `local` with no prefix and no owner.
    pub fn is_plain(&self, local: &str) -> bool {
        self.prefix.is_none() && self.owner.is_none() && self.local == local
    }
}

impl fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.prefix == Some(Prefix::Directive) {
            f.write_str("x:")?;
        }
        if let Some(owner) = &self.owner {
            write!(f, "{owner}.")?;
        }
        f.write_str(&self.local)
    }
}

/// `[A-Za-z_][A-Za-z0-9_-]*`.
pub(crate) fn is_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(is_identifier_byte)
}

/// A byte that may continue an identifier.
pub(crate) fn is_identifier_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_name_round_trips() {
        let name = QualifiedName::parse("Entity").unwrap();
        assert!(name.is_plain("Entity"));
        assert_eq!(name.to_string(), "Entity");
    }

    #[test]
    fn directive_and_owner_parts_are_split() {
        let name = QualifiedName::parse("x:Name").unwrap();
        assert_eq!(name.prefix(), Some(Prefix::Directive));
        assert_eq!(name.local(), "Name");
        let attached = QualifiedName::parse("Grid.Row").unwrap();
        assert_eq!(attached.owner(), Some("Grid"));
        assert_eq!(attached.local(), "Row");
        assert_eq!(attached.to_string(), "Grid.Row");
    }

    #[test]
    fn unknown_prefix_and_malformed_names_are_rejected() {
        assert_eq!(
            QualifiedName::parse("y:Name"),
            Err(QualifiedNameError::UnknownPrefix("y".to_string()))
        );
        for text in ["", "1a", "a.", ".a", "a.b.c", "x:"] {
            assert_eq!(
                QualifiedName::parse(text),
                Err(QualifiedNameError::Malformed),
                "{text}"
            );
        }
    }
}

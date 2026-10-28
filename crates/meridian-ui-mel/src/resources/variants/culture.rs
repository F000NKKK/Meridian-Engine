//! [`Culture`]: the language (and region) a resource variant is for.

use std::fmt;

/// A lowercase language tag: `ru`, `en-gb`. The tag after the set name in
/// `Strings.ru.mres`, and what the program selects at run time.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Culture(String);

impl Culture {
    /// `ru`, `RU`, `en-GB`: one to eight letters or digits per subtag,
    /// separated by `-`. `None` for anything else.
    pub fn parse(tag: &str) -> Option<Self> {
        let valid = tag.split('-').all(|part| {
            (1..=8).contains(&part.len()) && part.chars().all(|ch| ch.is_ascii_alphanumeric())
        });
        valid.then(|| Self(tag.to_ascii_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// This culture and the less specific ones it falls back to, most
    /// specific first: `en-gb` -> `en-gb`, `en`.
    pub fn chain(&self) -> Vec<Self> {
        let mut chain = vec![self.clone()];
        let mut tag = self.0.as_str();
        while let Some((parent, _)) = tag.rsplit_once('-') {
            chain.push(Self(parent.to_string()));
            tag = parent;
        }
        chain
    }
}

impl fmt::Display for Culture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_validated_and_lowercased() {
        assert_eq!(Culture::parse("en-GB").unwrap().as_str(), "en-gb");
        for bad in ["", "-", "en-", "en_gb", "toolongsubtag", "ру"] {
            assert_eq!(Culture::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_chain_goes_from_specific_to_general() {
        let chain: Vec<_> = Culture::parse("zh-hant-tw").unwrap().chain();
        let tags: Vec<_> = chain.iter().map(Culture::as_str).collect();
        assert_eq!(tags, ["zh-hant-tw", "zh-hant", "zh"]);
    }
}

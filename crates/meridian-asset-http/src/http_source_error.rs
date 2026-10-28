//! Why an [`HttpSource`](crate::HttpSource) could not be built.

/// A rejected base URL.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HttpSourceError {
    /// The URL starts with neither `http://` nor `https://`.
    #[error("only `http://` and `https://` base urls are supported, got `{0}`")]
    UnsupportedScheme(String),
    /// Nothing follows the scheme, or the host is empty.
    #[error("the base url `{0}` has no host")]
    MissingHost(String),
}

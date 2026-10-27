//! Why an [`HttpSource`](crate::HttpSource) could not be built.

/// A rejected base URL.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HttpSourceError {
    /// The URL does not start with `http://`. `https://` is refused on purpose:
    /// the build has no TLS (ADR 028), and a silent downgrade would be worse.
    #[error("only `http://` base urls are supported, got `{0}`")]
    UnsupportedScheme(String),
    /// Nothing follows `http://`, or the host is empty.
    #[error("the base url `{0}` has no host")]
    MissingHost(String),
}

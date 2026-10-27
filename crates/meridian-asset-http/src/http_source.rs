//! Asset bytes from a web server.

use std::time::Duration;

use meridian_asset_app::{AssetPath, ByteSource, SourceError};

use crate::HttpSourceError;
use crate::asset_url::asset_url;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_MAX_BYTES: u64 = 64 * 1024 * 1024;
const SCHEME: &str = "http://";

/// A [`ByteSource`] that reads `GET <base>/<asset path>`. It never follows a
/// redirect (an asset server that moves files should say so in its base URL),
/// gives up after a timeout, and refuses a body over a size cap, so a hostile
/// or broken server cannot hold a loader thread or fill memory.
#[derive(Debug, Clone)]
pub struct HttpSource {
    base: String,
    agent: ureq::Agent,
    max_bytes: u64,
}

impl HttpSource {
    /// A source below `base_url` (`http://host[:port][/prefix]`).
    ///
    /// # Errors
    ///
    /// [`HttpSourceError::UnsupportedScheme`] unless it starts with `http://`,
    /// [`HttpSourceError::MissingHost`] if no host follows.
    pub fn new(base_url: &str) -> Result<Self, HttpSourceError> {
        let rest = base_url
            .strip_prefix(SCHEME)
            .ok_or_else(|| HttpSourceError::UnsupportedScheme(base_url.to_owned()))?;
        if rest.is_empty() || rest.starts_with('/') {
            return Err(HttpSourceError::MissingHost(base_url.to_owned()));
        }
        Ok(Self {
            base: base_url.trim_end_matches('/').to_owned(),
            agent: build_agent(DEFAULT_TIMEOUT),
            max_bytes: DEFAULT_MAX_BYTES,
        })
    }

    /// Gives up on a request after `timeout` (connecting, waiting and reading together).
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.agent = build_agent(timeout);
        self
    }

    /// Refuses an asset larger than `max_bytes`.
    #[must_use]
    pub fn with_max_bytes(mut self, max_bytes: u64) -> Self {
        self.max_bytes = max_bytes;
        self
    }
}

fn build_agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .max_redirects(0)
        .http_status_as_error(false)
        .build()
        .into()
}

impl ByteSource for HttpSource {
    fn read(&self, path: &AssetPath) -> Result<Vec<u8>, SourceError> {
        let url = asset_url(&self.base, path).ok_or_else(|| SourceError::Rejected(path.clone()))?;
        let unreadable = |reason: String| SourceError::Unreadable {
            path: path.clone(),
            reason,
        };
        let mut response = self
            .agent
            .get(&url)
            .call()
            .map_err(|error| unreadable(error.to_string()))?;
        match response.status().as_u16() {
            200..=299 => response
                .body_mut()
                .with_config()
                .limit(self.max_bytes)
                .read_to_vec()
                .map_err(|error| unreadable(error.to_string())),
            404 | 410 => Err(SourceError::NotFound(path.clone())),
            status => Err(unreadable(format!("the server answered HTTP {status}"))),
        }
    }
}

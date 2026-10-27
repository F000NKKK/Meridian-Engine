//! Where an asset lives under a base URL.

use meridian_asset_app::AssetPath;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

/// Everything but the RFC 3986 unreserved characters is escaped.
const SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// `base` + `/` + the percent-encoded `path`, or `None` if the path could
/// name something else than a file below `base`: empty, an empty segment
/// (`a//b`, a leading or trailing `/`), or a `.` / `..` segment.
pub(crate) fn asset_url(base: &str, path: &AssetPath) -> Option<String> {
    let mut url = String::from(base);
    for segment in path.as_str().split('/') {
        if matches!(segment, "" | "." | "..") {
            return None;
        }
        url.push('/');
        url.extend(utf8_percent_encode(segment, SEGMENT));
    }
    Some(url)
}

# meridian-asset-http

HTTP adapter for `meridian-asset-app`: reads asset bytes from a web server.

**Layer and context:** infrastructure · assets

- **For:** `HttpSource`, a `ByteSource` rooted at a base URL (`http://host:port/assets`). Blocking, so it runs on the loader's background threads. Redirects are not followed, the body is capped (64 MiB unless raised) and a request times out (30 s unless changed); a path that is empty, `.`/`..` or has an empty segment is rejected, other bytes are percent-encoded.
- **Not for:** `https://` (needs a TLS library whose licence the workspace does not allow yet, ADR 028; `HttpSource::new` refuses it), decoding, or threads.

## Usage

```rust
use meridian_asset_app::{AssetPath, ByteSource, SourceError};
use meridian_asset_http::HttpSource;

let source = HttpSource::new("http://127.0.0.1:8080/assets").expect("http base url");
let escape = AssetPath::new("../secrets.txt");
assert_eq!(source.read(&escape), Err(SourceError::Rejected(escape)));
```

## Depends on / used by

- Depends on: [`meridian-asset-app`](../meridian-asset-app), `ureq` (without TLS), `percent-encoding`
- Used by: the composition root

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

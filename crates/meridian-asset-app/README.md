# meridian-asset-app

Asset loading ports: where bytes come from, where background work runs, and the loader that decodes off the frame thread.

**Layer and context:** application · assets

- **For:** `ByteSource` (read the bytes of an `AssetPath`), `Spawn` (run a job in the background), `AssetLoader` (read + decode as one background job) and `Pending<T>` (poll or wait for the result), and `PackSource`, a `.mpak` served as a `ByteSource`. ADR 027.
- **Not for:** decoding (the `Decoder` port is `meridian-asset-domain`), file access (`meridian-asset-fs`), threads (`meridian-runtime-threads`), or deciding how long a loaded asset lives (`meridian-resource`).

## Usage

```rust
use std::sync::Arc;

use meridian_asset_app::{AssetLoader, AssetPath, ByteSource, Priority, SourceError, Spawn};
use meridian_asset_domain::Decoder;

struct Memory(Vec<u8>);
impl ByteSource for Memory {
    fn read(&self, _path: &AssetPath) -> Result<Vec<u8>, SourceError> {
        Ok(self.0.clone())
    }
}

/// Runs the job right away on the caller; a real adapter uses a thread pool.
struct Inline;
impl Spawn for Inline {
    fn spawn(&self, _priority: Priority, job: Box<dyn FnOnce() + Send>) {
        job();
    }
}

struct Length;
impl Decoder<usize> for Length {
    type Error = std::convert::Infallible;
    fn decode(&self, bytes: &[u8]) -> Result<usize, Self::Error> {
        Ok(bytes.len())
    }
}

let loader = AssetLoader::new(Arc::new(Memory(vec![1, 2, 3])), Arc::new(Inline));
let pending = loader.load(AssetPath::new("a.bin"), Length);
assert_eq!(pending.wait().unwrap(), 3);
```

## Depends on / used by

- Depends on: [`meridian-asset-domain`](../meridian-asset-domain)
- Used by: [`meridian-asset-fs`](../meridian-asset-fs), [`meridian-runtime-threads`](../meridian-runtime-threads)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

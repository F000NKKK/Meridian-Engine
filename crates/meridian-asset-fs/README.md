# meridian-asset-fs

File-system adapter for `meridian-asset-app`: reads asset bytes from a directory.

**Layer and context:** infrastructure · assets

- **For:** `FsSource`, a `ByteSource` rooted at a directory. A path that would leave the root (`..`, an absolute path) is rejected, not read.
- **Not for:** decoding, threads, or network sources; the port and the loader are `meridian-asset-app`.

## Usage

```rust
use meridian_asset_app::{AssetPath, ByteSource, SourceError};
use meridian_asset_fs::FsSource;

let source = FsSource::new(std::env::temp_dir());
let escape = AssetPath::new("../secrets.txt");
assert_eq!(source.read(&escape), Err(SourceError::Rejected(escape)));
```

## Depends on / used by

- Depends on: [`meridian-asset-app`](../meridian-asset-app)
- Used by: the composition root, [`meridian-sdk`](../meridian-sdk) later

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

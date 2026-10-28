# meridian-asset-jpeg

JPEG adapter for `meridian-asset-domain`: decodes JPEG bytes to RGBA8 on the `zune-jpeg` crate.

**Layer and context:** infrastructure · assets

- **For:** `JpegDecoder`, a `Decoder<ImageAsset>` for baseline and progressive JPEG; images above 16384 pixels a side are refused (ADR 013: codecs are adapters).
- **Not for:** the `Decoder` port, `ImageAsset`, format detection: those are `meridian-asset-domain`.

## Usage

```rust
use meridian_asset_domain::Decoder;
use meridian_asset_jpeg::JpegDecoder;

// Bytes without the JPEG marker are rejected with a typed error.
assert!(JpegDecoder.decode(b"not a jpeg file at all!!").is_err());
```

## Depends on / used by

- Depends on: [`meridian-asset-domain`](../meridian-asset-domain)
- Used by: [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

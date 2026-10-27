# meridian-asset-png

PNG adapter for `meridian-asset-domain`: decodes PNG bytes to RGBA8 on the `png` crate.

**Layer and context:** infrastructure · assets

- **For:** `PngDecoder`, a `Decoder<ImageAsset>` that expands palette, grayscale and 16-bit sources to RGBA8 (ADR 013: DEFLATE is not hand-rolled).
- **Not for:** the `Decoder` port, `ImageAsset`, BMP and format detection: those are `meridian-asset-domain`.

## Usage

```rust
use meridian_asset_domain::Decoder;
use meridian_asset_png::PngDecoder;

// Bytes without the PNG signature are rejected with a typed error.
assert!(PngDecoder.decode(b"not a png file at all!!").is_err());
```

## Depends on / used by

- Depends on: [`meridian-asset-domain`](../meridian-asset-domain)
- Used by: [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

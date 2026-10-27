//! The behaviour every [`TextureStore`](crate::TextureStore) must have.

use meridian_asset_core::ImageAsset;

use crate::TextureStore;

/// Asserts the texture store contract against fresh stores from `make`: an
/// upload yields a live handle, handles are distinct, `release` frees exactly
/// once, and a released handle is no longer live.
///
/// # Panics
///
/// Panics when `S` breaks the contract.
pub fn run_texture_store_contract<S: TextureStore>(make: impl Fn() -> S) {
    let mut store = make();
    let image = ImageAsset {
        width: 2,
        height: 2,
        pixels: vec![255; 16],
    };
    let first = store.upload(&image);
    let second = store.upload_solid([255, 0, 0, 255]);
    assert!(
        store.contains(first) && store.contains(second),
        "uploads are live"
    );
    assert_ne!(first, second, "handles are distinct");

    assert!(store.release(first), "release frees a live texture");
    assert!(!store.contains(first), "a released handle is dead");
    assert!(!store.release(first), "release frees exactly once");
    assert!(store.contains(second), "other textures are untouched");

    let other = make();
    assert!(!other.contains(second), "stores are independent");
}

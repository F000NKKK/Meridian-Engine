//! Where each shadow tile sits in the atlas (ADR 043, ADR 045).

use meridian_rendering_domain::{MAX_LOCAL_SHADOWS, POINT_SHADOW_FACES, SHADOW_CASCADES};

/// Tiles of the atlas: the four cascades, then the six faces of each slot's
/// point light cube, slot after slot, then one tile per slot for a spot light.
pub(crate) const SHADOW_TILES: usize =
    SHADOW_CASCADES + MAX_LOCAL_SHADOWS * POINT_SHADOW_FACES + MAX_LOCAL_SHADOWS;
/// Cascade tiles across the atlas.
pub(crate) const ATLAS_COLUMNS: u32 = 2;
/// Cascade tiles down the atlas: two for the cascades, three for the point
/// light blocks (see [`tile_rect`]).
pub(crate) const ATLAS_ROWS: u32 = 5;

/// A tile's rectangle in texels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TileRect {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) size: u32,
}

/// The index of face `face` of the `slot`th point light's cube.
pub(crate) const fn face_tile(slot: usize, face: usize) -> usize {
    SHADOW_CASCADES + slot * POINT_SHADOW_FACES + face
}

/// The index of the tile of the spot light in `slot`.
pub(crate) const fn spot_tile(slot: usize) -> usize {
    SHADOW_CASCADES + MAX_LOCAL_SHADOWS * POINT_SHADOW_FACES + slot
}

/// The rectangle of tile `index` in an atlas of cascade tiles of `tile`
/// texels. Cascades sit in a 2 x 2 grid on top. Below them every point light
/// owns a block of 2 x 3 faces, each half a cascade tile wide; the blocks are
/// laid out two across, so the whole atlas is `ATLAS_COLUMNS * tile` by
/// `ATLAS_ROWS * tile` texels. A spot light's tile is a full cascade tile at
/// the origin of its slot's block, over the faces a point light would use
/// there; a slot holds one kind of light at a time. The shader repeats this
/// arithmetic.
pub(crate) fn tile_rect(index: usize, tile: u32) -> TileRect {
    let column = |i: usize| u32::try_from(i % 2).unwrap_or(0);
    let row = |i: usize| u32::try_from(i / 2).unwrap_or(0);
    if index < SHADOW_CASCADES {
        return TileRect {
            x: column(index) * tile,
            y: row(index) * tile,
            size: tile,
        };
    }
    let size = tile / 2;
    let spots = SHADOW_CASCADES + MAX_LOCAL_SHADOWS * POINT_SHADOW_FACES;
    if index >= spots {
        let slot = index - spots;
        return TileRect {
            x: column(slot) * tile,
            y: 2 * tile + row(slot) * 3 * size,
            size: tile,
        };
    }
    let (slot, face) = (
        (index - SHADOW_CASCADES) / POINT_SHADOW_FACES,
        (index - SHADOW_CASCADES) % POINT_SHADOW_FACES,
    );
    TileRect {
        x: column(slot) * tile + column(face) * size,
        y: 2 * tile + row(slot) * 3 * size + row(face) * size,
        size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cascades_fill_the_top_two_rows_of_the_atlas() {
        assert_eq!(
            tile_rect(0, 1024),
            TileRect {
                x: 0,
                y: 0,
                size: 1024
            }
        );
        assert_eq!(
            tile_rect(3, 1024),
            TileRect {
                x: 1024,
                y: 1024,
                size: 1024
            }
        );
    }

    #[test]
    fn point_light_faces_fill_the_rest_without_overlap() {
        let faces = SHADOW_TILES - MAX_LOCAL_SHADOWS;
        let rects: Vec<TileRect> = (0..faces).map(|i| tile_rect(i, 1024)).collect();
        let (width, height) = (ATLAS_COLUMNS * 1024, ATLAS_ROWS * 1024);
        for (i, a) in rects.iter().enumerate() {
            assert!(
                a.x + a.size <= width && a.y + a.size <= height,
                "tile {i} leaves the atlas"
            );
            for (j, b) in rects.iter().enumerate().skip(i + 1) {
                let apart = a.x + a.size <= b.x
                    || b.x + b.size <= a.x
                    || a.y + a.size <= b.y
                    || b.y + b.size <= a.y;
                assert!(apart, "tiles {i} and {j} overlap");
            }
        }
    }

    #[test]
    fn a_spot_tile_sits_over_its_slots_block_inside_the_atlas() {
        for slot in 0..MAX_LOCAL_SHADOWS {
            let spot = tile_rect(spot_tile(slot), 1024);
            let first_face = tile_rect(face_tile(slot, 0), 1024);
            assert_eq!((spot.x, spot.y), (first_face.x, first_face.y));
            assert_eq!(spot.size, 1024);
            assert!(spot.x + spot.size <= ATLAS_COLUMNS * 1024);
            assert!(spot.y + spot.size <= ATLAS_ROWS * 1024);
        }
    }

    #[test]
    fn the_fourth_light_ends_at_the_bottom_of_the_atlas() {
        let last = tile_rect(face_tile(3, 5), 1024);
        assert_eq!(last.y + last.size, ATLAS_ROWS * 1024);
    }
}

//! Where each shadow tile sits in the atlas (ADR 043).

use meridian_rendering_domain::{MAX_POINT_SHADOWS, POINT_SHADOW_FACES, SHADOW_CASCADES};

/// Tiles of the atlas: the four cascades, then the six faces of each point
/// light's cube, light after light.
pub(crate) const SHADOW_TILES: usize = SHADOW_CASCADES + MAX_POINT_SHADOWS * POINT_SHADOW_FACES;
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

/// The rectangle of tile `index` in an atlas of cascade tiles of `tile`
/// texels. Cascades sit in a 2 x 2 grid on top. Below them every point light
/// owns a block of 2 x 3 faces, each half a cascade tile wide; the blocks are
/// laid out two across, so the whole atlas is `ATLAS_COLUMNS * tile` by
/// `ATLAS_ROWS * tile` texels. The shader repeats this arithmetic.
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
    let (slot, face) = (
        (index - SHADOW_CASCADES) / POINT_SHADOW_FACES,
        (index - SHADOW_CASCADES) % POINT_SHADOW_FACES,
    );
    let size = tile / 2;
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
        let rects: Vec<TileRect> = (0..SHADOW_TILES).map(|i| tile_rect(i, 1024)).collect();
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
    fn the_fourth_light_ends_at_the_bottom_of_the_atlas() {
        let last = tile_rect(face_tile(3, 5), 1024);
        assert_eq!(last.y + last.size, ATLAS_ROWS * 1024);
    }
}

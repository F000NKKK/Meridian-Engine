# Texture orientation

UV coordinates name positions in an image, not world axes. A world-space view
also depends on mesh UVs, the object's pose and the side from which it is viewed.

| Image corner | UV |
| --- | --- |
| Top left | (0, 0) |
| Top right | (1, 0) |
| Bottom left | (0, 1) |
| Bottom right | (1, 1) |

PNG decoding and BMP conversion produce top-to-bottom RGBA rows. Material upload
preserves that order. Mesh UVs pass through the material shader unchanged; the
glTF importer preserves authored TEXCOORD_0 rather than inserting a V flip.
Do not add a second vertical flip merely because a source uses OpenGL conventions.
At repeat-addressed edges 1 wraps to 0; inspect texel centers rather than an exact
UV border when checking corner colors.

The procedural ground has U increasing along local +X, V along local +Z, with
(0,0) at (-half_size, 0, -half_size). Its front is viewed from +Y. Rotating the
ground rotates these directions in the world.

The procedural sphere uses equirectangular UVs: V=0 at +Y, V=1 at -Y;
U=0.5 at +X and increases towards +Z. Its seam is at -X. Seam vertices have
separate UVs so a triangle interpolates over the shortest longitude interval.
Some U values exceed 1 deliberately and require repeat addressing. Pole vertices
use their incident triangle's longitude rather than imposing an arbitrary roll.
This fixes smear near the seam; it does not remove inherent equirectangular pole
distortion or implement arbitrary glTF sampler/texture-transform extensions.

Regression: `cargo test -p meridian-rendering-domain --test sphere_uv` checks
subdivision levels 0 through 3 and preserves positions and normals when splitting
UV vertices. It is a geometry test, not a pixel-rendering test. A labeled corner
texture and an end-to-end rendered importer fixture remain follow-up work.

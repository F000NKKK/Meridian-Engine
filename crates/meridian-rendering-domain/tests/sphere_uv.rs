//! Sphere textures must not smear across their longitude seam.
use meridian_rendering_domain::icosphere_mesh_source;

#[test]
fn each_sphere_triangle_interpolates_the_short_texture_interval() {
    for subdivisions in 0..=3 {
        let sphere = icosphere_mesh_source(subdivisions, 2.0);
        for face in sphere.indices.as_chunks::<3>().0 {
            let coordinates: Vec<_> = face.iter().map(|i| sphere.uvs[*i as usize]).collect();
            let minimum = coordinates
                .iter()
                .map(|uv| uv[0])
                .fold(f32::INFINITY, f32::min);
            let maximum = coordinates
                .iter()
                .map(|uv| uv[0])
                .fold(f32::NEG_INFINITY, f32::max);
            assert!(
                maximum - minimum <= 0.5 + 1e-6,
                "subdivisions {subdivisions}: {coordinates:?}"
            );
            for index in face {
                let p = sphere.positions[*index as usize];
                let n = sphere.normals[*index as usize];
                assert!((p[0] - n[0] * 2.0).abs() < 1e-6);
                assert!((p[1] - n[1] * 2.0).abs() < 1e-6);
                assert!((p[2] - n[2] * 2.0).abs() < 1e-6);
            }
        }
    }
}

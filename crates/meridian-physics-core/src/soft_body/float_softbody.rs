//! Thin `FloatFlavor` (`f32`) aliases over `super::generic_softbody`'s
//! soft-body engine — the default, everyday deformable-body path. See
//! `crate::float`'s module doc for why this is just aliases, not a
//! second copy of the engine.

pub use meridian_gac_core::float_ga::FloatFlavor;

pub type Spring = super::generic_softbody::Spring<FloatFlavor>;
pub type SoftBody = super::generic_softbody::SoftBody<FloatFlavor>;
pub type SoftBodyIntegrator = super::generic_softbody::SoftBodyIntegrator<FloatFlavor>;

pub fn icosphere_soft_body(
    center: meridian_gac_core::Vec3,
    radius: f32,
    subdivisions: u32,
    particle_mass: f32,
    edge_stiffness: f32,
    edge_damping: f32,
    spoke_stiffness: f32,
    spoke_damping: f32,
) -> SoftBody {
    super::generic_softbody::icosphere_soft_body::<FloatFlavor>(
        center,
        radius,
        subdivisions,
        particle_mass,
        edge_stiffness,
        edge_damping,
        spoke_stiffness,
        spoke_damping,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_gac_core::{Plane, Vec3};

    fn ground() -> Plane {
        Plane {
            normal: Vec3::Y,
            d: 0.0,
        }
        .normalize()
    }

    fn ball(center: Vec3, radius: f32) -> SoftBody {
        icosphere_soft_body(center, radius, 1, 0.05, 400.0, 2.0, 150.0, 1.0)
    }

    #[test]
    fn icosphere_soft_body_places_surface_particles_at_requested_radius() {
        let body = ball(Vec3::new(0.0, 5.0, 0.0), 1.0);
        let center_index = body.particle_count() - 1;
        for (i, &p) in body.positions.iter().enumerate() {
            if i == center_index {
                continue;
            }
            let dist = (p - body.positions[center_index]).length();
            assert!(
                (dist - 1.0).abs() < 1e-4,
                "surface particle {i} at distance {dist}, expected 1.0"
            );
        }
    }

    #[test]
    fn ball_falls_under_gravity() {
        let mut body = ball(Vec3::new(0.0, 5.0, 0.0), 0.5);
        let integrator = SoftBodyIntegrator::new(Vec3::new(0.0, -9.81, 0.0), ground(), 0.3);
        let center_index = body.particle_count() - 1;
        let start_height = body.positions[center_index].y;
        for _ in 0..30 {
            integrator.step(&mut body, 1.0 / 60.0);
        }
        assert!(
            body.positions[center_index].y < start_height,
            "the ball's center must have fallen"
        );
    }

    /// The actual point of this whole module: a ball dropped onto the
    /// ground must visibly *deform* on impact (its vertical extent
    /// shrinks below its resting diameter at the moment of peak
    /// compression) and then recover close to its original shape — not
    /// stay perfectly rigid (that would mean the springs aren't doing
    /// anything) and not stay permanently squashed (that would mean it's
    /// not a spring system, just inelastic collapse).
    #[test]
    fn ball_deforms_on_impact_and_recovers_its_shape() {
        let radius = 0.5;
        let mut body = ball(Vec3::new(0.0, radius + 0.01, 0.0), radius);
        let integrator = SoftBodyIntegrator::new(Vec3::new(0.0, -9.81 * 4.0, 0.0), ground(), 0.2);
        let dt = 1.0 / 240.0;

        let vertical_extent = |body: &SoftBody| -> f32 {
            let min = body
                .positions
                .iter()
                .map(|p| p.y)
                .fold(f32::INFINITY, f32::min);
            let max = body
                .positions
                .iter()
                .map(|p| p.y)
                .fold(f32::NEG_INFINITY, f32::max);
            max - min
        };

        let resting_extent = vertical_extent(&body);
        let mut min_extent_during_impact = f32::INFINITY;
        for _ in 0..600 {
            integrator.step(&mut body, dt);
            min_extent_during_impact = min_extent_during_impact.min(vertical_extent(&body));
        }

        assert!(
            min_extent_during_impact < resting_extent * 0.9,
            "ball must visibly compress on impact: resting extent {resting_extent}, min during impact {min_extent_during_impact}"
        );

        // Let it settle further and check it recovered a sane resting
        // shape (not collapsed, not exploded) rather than checking an
        // exact final extent, which would be sensitive to the exact
        // damping tuning above.
        for _ in 0..600 {
            integrator.step(&mut body, dt);
        }
        let final_extent = vertical_extent(&body);
        assert!(
            final_extent > resting_extent * 0.5 && final_extent < resting_extent * 1.5,
            "ball must recover close to its resting shape, got extent {final_extent} vs resting {resting_extent}"
        );
    }

    #[test]
    fn pinned_particle_never_moves() {
        let mut body = ball(Vec3::new(0.0, 5.0, 0.0), 0.5);
        body.inverse_masses[0] = 0.0;
        let pinned_start = body.positions[0];
        let integrator = SoftBodyIntegrator::new(Vec3::new(0.0, -9.81, 0.0), ground(), 0.3);
        for _ in 0..120 {
            integrator.step(&mut body, 1.0 / 60.0);
        }
        assert_eq!(
            body.positions[0], pinned_start,
            "a pinned particle (inverse_mass = 0) must never move"
        );
    }
}

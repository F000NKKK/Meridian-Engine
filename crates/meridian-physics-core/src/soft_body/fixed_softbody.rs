//! Thin `FixedFlavor` (`Fixed`-point) aliases over
//! `super::generic_softbody`'s soft-body engine — an opt-in,
//! bit-reproducible deformable-body path for lockstep networking/replay,
//! the same role `crate::fixed` plays for the rigid-body engine. See
//! `crate::fixed`'s module doc for the determinism rationale;
//! `Fixed::sqrt` (used for every spring's current length each step) is
//! already exact/deterministic on CPU — see
//! `meridian_numeric_core::Fixed`'s own doc comment — so this needed no
//! extra work beyond aliasing `generic_softbody<FixedFlavor>`.

pub use meridian_gac_core::fixed_ga::FixedFlavor;

pub type FixedSpring = super::generic_softbody::Spring<FixedFlavor>;
pub type FixedSoftBody = super::generic_softbody::SoftBody<FixedFlavor>;
pub type FixedSoftBodyIntegrator = super::generic_softbody::SoftBodyIntegrator<FixedFlavor>;

pub fn fixed_icosphere_soft_body(
    center: meridian_gac_core::fixed_ga::FixedVec3,
    radius: meridian_numeric_core::Fixed,
    subdivisions: u32,
    particle_mass: meridian_numeric_core::Fixed,
    edge_stiffness: meridian_numeric_core::Fixed,
    edge_damping: meridian_numeric_core::Fixed,
    spoke_stiffness: meridian_numeric_core::Fixed,
    spoke_damping: meridian_numeric_core::Fixed,
) -> FixedSoftBody {
    super::generic_softbody::icosphere_soft_body::<FixedFlavor>(
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
    use meridian_gac_core::fixed_ga::FixedVec3;
    use meridian_gac_core::generic::Plane;
    use meridian_numeric_core::Fixed;

    fn fv3(x: f64, y: f64, z: f64) -> FixedVec3 {
        FixedVec3::new(Fixed::from_num(x), Fixed::from_num(y), Fixed::from_num(z))
    }

    fn ground() -> Plane<FixedFlavor> {
        Plane {
            normal: fv3(0.0, 1.0, 0.0),
            d: Fixed::ZERO,
        }
        .normalize()
    }

    fn ball(center: FixedVec3, radius: f64) -> FixedSoftBody {
        fixed_icosphere_soft_body(
            center,
            Fixed::from_num(radius),
            1,
            Fixed::from_num(0.05),
            Fixed::from_num(400.0),
            Fixed::from_num(2.0),
            Fixed::from_num(150.0),
            Fixed::from_num(1.0),
        )
    }

    #[test]
    fn ball_falls_under_gravity() {
        let mut body = ball(fv3(0.0, 5.0, 0.0), 0.5);
        let integrator = FixedSoftBodyIntegrator::new(fv3(0.0, -9.81, 0.0), ground(), Fixed::from_num(0.3));
        let center_index = body.particle_count() - 1;
        let start_height = body.positions[center_index].y;
        let dt = Fixed::from_num(1.0 / 60.0);
        for _ in 0..30 {
            integrator.step(&mut body, dt);
        }
        assert!(
            body.positions[center_index].y < start_height,
            "the ball's center must have fallen"
        );
    }

    /// The same bit-exact reproducibility proof `physics-core::fixed`'s
    /// rigid-body path already has, applied to the soft-body path: two
    /// independent runs of the identical scenario (gravity, spring
    /// forces, ground-plane collision — every piece of the pipeline)
    /// must produce identical `Fixed` bit patterns, not just
    /// approximately-equal floats.
    #[test]
    fn identical_inputs_produce_bit_identical_output_after_many_steps() {
        fn run() -> FixedSoftBody {
            // A gentle drop, not a big one: explicit-Euler mass-spring
            // integration is only conditionally stable (stiffness/mass/dt
            // have to stay in a sane relationship — the same reason the
            // `float_softbody` deformation test uses a barely-above-ground
            // start rather than a multi-meter fall). This test's job is
            // proving bit-exact reproducibility, not stress-testing spring
            // stability, so it deliberately stays inside the stable
            // regime.
            let mut body = ball(fv3(0.3, 0.7, -0.2), 0.5);
            let integrator =
                FixedSoftBodyIntegrator::new(fv3(0.0, -9.81, 0.0), ground(), Fixed::from_num(0.3));
            let dt = Fixed::from_num(1.0 / 60.0);
            for _ in 0..180 {
                integrator.step(&mut body, dt);
            }
            body
        }

        let run_a = run();
        let run_b = run();
        assert_eq!(
            run_a.positions, run_b.positions,
            "identical deterministic soft-body simulations must produce bit-identical positions"
        );
        assert_eq!(
            run_a.velocities, run_b.velocities,
            "identical deterministic soft-body simulations must produce bit-identical velocities"
        );
    }
}

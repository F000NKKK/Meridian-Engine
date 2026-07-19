//! Roadmap milestone: validate `physics-core::deterministic`'s opt-in,
//! bit-reproducible simulation path — `Fixed` (Q16.16 fixed-point, CORDIC
//! trig, integer sqrt) instead of `f32`, for lockstep networking/replay
//! (see `meridian_numeric_core::Fixed`'s doc comment for why `f32` can't
//! promise this). The exhaustive checks live in
//! `meridian-physics-core::deterministic`'s own test suite (`cargo test
//! -p meridian-physics-core`); this is the human-readable version.
//!
//! Run with:
//!   ./build.sh run determinism_validation

use meridian_gac_core::fixed_ga::{FixedMotor3, FixedVec3};
use meridian_numeric_core::Fixed;
use meridian_physics_core::deterministic::{
    DeterministicBody, DeterministicBroadPhase, DeterministicConstraintSolver,
    DeterministicIntegrator, DeterministicNarrowPhase, DeterministicShape,
};

fn check(label: &str, condition: bool) {
    println!("  [{}] {label}", if condition { "OK" } else { "FAIL" });
    assert!(condition, "{label} failed");
}

fn fv3(x: f64, y: f64, z: f64) -> FixedVec3 {
    FixedVec3::new(Fixed::from_num(x), Fixed::from_num(y), Fixed::from_num(z))
}

fn sphere(position: FixedVec3, velocity: FixedVec3, mass: f64, radius: f64) -> DeterministicBody {
    DeterministicBody {
        frame: FixedMotor3::translation(position),
        velocity,
        mass: Fixed::from_num(mass),
        shape: DeterministicShape::Sphere {
            radius: Fixed::from_num(radius),
        },
        ..Default::default()
    }
}

/// A scenario with everything the pipeline touches: gravity, a
/// collision with restitution, positional correction — run identically
/// twice to prove reproducibility. A single dynamic ball directly above
/// the floor's pole (the only *stable* resting point on a curved
/// "floor" — this is genuinely a giant sphere, not a flat plane, so any
/// horizontal offset is an unstable-equilibrium starting point with this
/// frictionless solver: the ball would slide/roll indefinitely down the
/// curve, which is correct physics but a confusing, hard-to-eyeball
/// example) settling to rest, matching `physics-core`'s own proven
/// `full_step_ball_settles_on_static_floor_without_sinking_through` test
/// scenario exactly.
fn run_scenario() -> Vec<DeterministicBody> {
    let mut bodies = vec![
        sphere(fv3(0.0, -50.0, 0.0), FixedVec3::ZERO, 0.0, 50.0), // static floor
        sphere(fv3(0.0, 3.0, 0.0), FixedVec3::ZERO, 1.0, 0.5),
    ];
    let integrator = DeterministicIntegrator::default();
    let solver = DeterministicConstraintSolver::new(Fixed::from_num(0.3));
    let mut broad = DeterministicBroadPhase::new();
    let narrow = DeterministicNarrowPhase::new();
    let dt = Fixed::from_num(1.0 / 60.0);

    for _ in 0..300 {
        integrator.step(&mut bodies, dt);
        let pairs = broad.find_candidate_pairs(&bodies).to_vec();
        for contact in narrow.generate_contacts(&bodies, &pairs) {
            solver.resolve(&mut bodies, &contact);
        }
    }
    bodies
}

fn main() {
    println!("== Fixed: deterministic arithmetic, cross-checked against f64 ==");
    let angle = Fixed::from_num(std::f64::consts::FRAC_PI_4);
    let (sin, cos) = angle.sin_cos();
    println!(
        "  CORDIC sin(pi/4)={:.5} cos(pi/4)={:.5} (f64 oracle: {:.5}/{:.5})",
        sin.to_num(),
        cos.to_num(),
        std::f64::consts::FRAC_PI_4.sin(),
        std::f64::consts::FRAC_PI_4.cos()
    );
    check(
        "CORDIC sin/cos match the f64 oracle within Q16.16 precision",
        (sin.to_num() - std::f64::consts::FRAC_PI_4.sin()).abs() < 1e-3
            && (cos.to_num() - std::f64::consts::FRAC_PI_4.cos()).abs() < 1e-3,
    );

    println!("\n== DeterministicBody: full physics pipeline, run twice ==");
    let run_a = run_scenario();
    let run_b = run_scenario();

    for (i, (a, b)) in run_a.iter().zip(run_b.iter()).enumerate() {
        println!(
            "  body {i}: run A y={:.6}  run B y={:.6}  (raw bits: {} vs {})",
            a.position().y.to_num(),
            b.position().y.to_num(),
            a.position().y.to_bits(),
            b.position().y.to_bits()
        );
    }
    check(
        "two independent runs of the same scenario are bit-for-bit identical",
        run_a == run_b,
    );

    println!("\n== Handoff to rendering: Fixed pose -> f32 Motor3 ==");
    let f32_frame = run_a[1].frame_f32();
    let f32_position = f32_frame.transform_point(meridian_gac_core::Vec3::ZERO);
    let fixed_position = run_a[1].position();
    println!(
        "  Fixed position (exact): ({:.6}, {:.6}, {:.6})",
        fixed_position.x.to_num(),
        fixed_position.y.to_num(),
        fixed_position.z.to_num()
    );
    println!("  f32 Motor3 position (for rendering): {f32_position:?}");
    check(
        "f32 handoff position matches the Fixed source within f32 precision",
        (f32_position.x - fixed_position.x.to_num() as f32).abs() < 1e-3
            && (f32_position.y - fixed_position.y.to_num() as f32).abs() < 1e-3
            && (f32_position.z - fixed_position.z.to_num() as f32).abs() < 1e-3,
    );

    println!(
        "\nAll checks passed — Fixed-point math and the deterministic physics pipeline reproduce bit-exactly."
    );
}

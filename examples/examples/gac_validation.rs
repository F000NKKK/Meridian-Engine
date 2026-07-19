//! Roadmap milestone: validate that `meridian-gac-core`'s `Motor3` API is
//! usable and correct before anything is built on top of it (ecs-core,
//! physics-core, graphics-core). See docs/roadmap.md step 2.
//!
//! The exhaustive numeric checks live in `meridian-gac-core`'s own test
//! suite (`cargo test -p meridian-gac-core`); this example is the
//! human-readable version — run it and read the output to see the same
//! properties hold.
//!
//! Run with:
//!   ./build.sh run gac_validation

use meridian_gac_core::{Motor3, Rotor, Vec3};
use std::f32::consts::PI;

fn approx_eq(a: Vec3, b: Vec3) -> bool {
    let d = a - b;
    d.length() < 1e-4
}

fn check(label: &str, got: Vec3, want: Vec3) {
    let ok = approx_eq(got, want);
    println!(
        "  [{}] {label}: got {got:?}, want {want:?}",
        if ok { "OK" } else { "FAIL" }
    );
    assert!(ok, "{label} failed: got {got:?}, want {want:?}");
}

fn main() {
    println!("Vec3 basics");
    let a = Vec3::new(1.0, 0.0, 0.0);
    let b = Vec3::new(0.0, 1.0, 0.0);
    check("cross(X, Y) == Z", a.cross(b), Vec3::Z);
    println!("  dot(X, Y) = {}", a.dot(b));

    println!("\nRotor: 90 degree turn about Z maps X -> Y");
    let quarter_turn = Rotor::from_axis_angle(Vec3::Z, PI / 2.0);
    check(
        "rotate X about Z by 90deg",
        quarter_turn.transform_vector(Vec3::X),
        Vec3::Y,
    );

    println!("\nRotor composition: two half-turns == one full turn");
    let half = Rotor::from_axis_angle(Vec3::Z, PI / 2.0);
    let composed = half.compose(half);
    let full = Rotor::from_axis_angle(Vec3::Z, PI);
    check(
        "compose(90deg, 90deg) == 180deg",
        composed.transform_vector(Vec3::X),
        full.transform_vector(Vec3::X),
    );

    println!("\nMotor3: rotate then translate");
    let motor = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::Z, PI / 2.0),
        Vec3::new(5.0, 0.0, 0.0),
    );
    // X (1,0,0) rotates to (0,1,0), then translates by (5,0,0) -> (5,1,0)
    check(
        "rotate(X, 90deg) + translate(5,0,0)",
        motor.transform_point(Vec3::X),
        Vec3::new(5.0, 1.0, 0.0),
    );

    println!("\nMotor3 inverse round-trips a point");
    let p = Vec3::new(3.0, -2.0, 7.0);
    let round_tripped = motor.inverse().transform_point(motor.transform_point(p));
    check(
        "inverse(motor).transform(motor.transform(p)) == p",
        round_tripped,
        p,
    );

    println!("\nParent/child transform hierarchy propagation");
    let parent = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::Z, PI / 4.0),
        Vec3::new(10.0, 0.0, 0.0),
    );
    let child = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::Y, PI / 6.0),
        Vec3::new(0.0, 2.0, 0.0),
    );
    let local_point = Vec3::new(1.0, 0.0, 0.0);

    let world_motor = child.compose(parent);
    let via_composed = world_motor.transform_point(local_point);
    let via_steps = parent.transform_point(child.transform_point(local_point));
    check(
        "composed(child, parent).transform(p) == parent.transform(child.transform(p))",
        via_composed,
        via_steps,
    );

    println!("\nAll checks passed.");
}

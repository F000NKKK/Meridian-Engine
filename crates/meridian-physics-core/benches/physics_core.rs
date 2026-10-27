//! Criterion benches for the rigid-body step: integrate, broad phase,
//! narrow phase and resolution.

use criterion::{Criterion, criterion_group, criterion_main};
use meridian_gac_core::{Motor3, Vec3};
use meridian_physics_core::{
    BroadPhase, ColliderShape, ConstraintSolver, Integrator, NarrowPhase, RigidBody,
};
use std::hint::black_box;

fn grid_of_spheres(side: u16) -> Vec<RigidBody> {
    let mut bodies = Vec::with_capacity(usize::from(side) * usize::from(side));
    for x in 0..side {
        for z in 0..side {
            bodies.push(RigidBody {
                frame: Motor3::translation(Vec3::new(f32::from(x) * 0.9, 0.0, f32::from(z) * 0.9)),
                velocity: Vec3::new(0.0, -1.0, 0.0),
                mass: 1.0,
                shape: ColliderShape::Sphere { radius: 0.5 },
                ..Default::default()
            });
        }
    }
    bodies
}

fn integrate(c: &mut Criterion) {
    let integrator = Integrator::new(Vec3::new(0.0, -9.81, 0.0));
    let mut bodies = grid_of_spheres(32);
    c.bench_function("integrator_step_1024", |b| {
        b.iter(|| integrator.step(black_box(&mut bodies), 1.0 / 60.0));
    });
}

fn collide_and_resolve(c: &mut Criterion) {
    let bodies = grid_of_spheres(16);
    let mut broad = BroadPhase::new();
    let narrow = NarrowPhase::new();
    let solver = ConstraintSolver::new(0.3);
    c.bench_function("collide_and_resolve_256", |b| {
        b.iter(|| {
            let mut scratch = bodies.clone();
            let pairs = broad.find_candidate_pairs(&scratch).to_vec();
            for contact in narrow.generate_contacts(&scratch, &pairs) {
                solver.resolve(&mut scratch, &contact);
            }
            black_box(scratch)
        });
    });
}

criterion_group!(benches, integrate, collide_and_resolve);
criterion_main!(benches);

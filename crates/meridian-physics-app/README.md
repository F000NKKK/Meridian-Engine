# meridian-physics-app

Physics use cases: the `PhysicsSubsystem` world state, the `PhysicsStepper` port and its sequential implementation.

**Layer and context:** application · physics

- **For:** the plain-data physics world (`PhysicsSubsystem`), the `PhysicsStepper` port that advances it by one timestep, `SequentialStepper` (a CPU loop), and a contract suite (`testing`) every stepper must pass.
- **Not for:** rigid-body algorithms, which are `meridian-physics-core`; batched or GPU stepping, which is `meridian-physics-compute`; deciding when a step runs, which is `meridian-engine-core`.

## Usage

```rust
use meridian_physics_app::{PhysicsStepper, PhysicsSubsystem, SequentialStepper};

let mut physics = PhysicsSubsystem::default();
SequentialStepper.step(&mut physics, 1.0 / 60.0);
assert!(physics.bodies.is_empty());
```

## Features

| Feature | Enables |
|---|---|
| `testing` | `testing::run_stepper_contract` for adapters' own tests |

## Depends on / used by

- Depends on: [`meridian-gac-core`](../meridian-gac-core), [`meridian-physics-core`](../meridian-physics-core)
- Used by: [`meridian-engine-core`](../meridian-engine-core), [`meridian-physics-compute`](../meridian-physics-compute), [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

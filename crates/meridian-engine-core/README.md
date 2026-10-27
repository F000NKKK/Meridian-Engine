# meridian-engine-core

Runtime use cases: stages run in dependency order through a job-executor port, the event system, and the physics and compute step stages.

**Layer and context:** application · runtime

- **For:** `Runtime` and `Stage`s run in dependency order through the `JobExecutor` port, `Shared` state handles, `EventSystem`, `PhysicsStepStage` and `ComputeStage`.
- **Not for:** owning application state or naming adapters (ADR 025): threads come from `meridian-task-core`, physics steppers from `meridian-physics-app` and `meridian-physics-compute`, subsystem algorithms from the `*-core` crates.

## Usage

```rust
use std::sync::Arc;

use meridian_engine_core::testing::InlineExecutor;
use meridian_engine_core::{Runtime, Shared, Stage};

struct Count(Shared<u32>);

impl Stage for Count {
    fn run(&mut self) {
        *self.0.lock() += 1;
    }
}

let frames = Shared::new(0);
let mut runtime = Runtime::new(Arc::new(InlineExecutor));
runtime.add_stage("count", &[], Count(frames.clone()));
runtime.tick();
assert_eq!(*frames.lock(), 1);
```

## Features

| Feature | Enables |
|---|---|
| `testing` | `testing::InlineExecutor` and `testing::run_executor_contract` for adapters' own tests |

## Depends on / used by

- Depends on: [`meridian-compute-runtime`](../meridian-compute-runtime), [`meridian-foundation`](../meridian-foundation), [`meridian-physics-app`](../meridian-physics-app)
- Used by: [`meridian-sdk`](../meridian-sdk), [`meridian-task-core`](../meridian-task-core)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

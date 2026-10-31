# SDK and CLI usability work

Requested scope, in implementation order:

- [x] Audit existing Rotor/Motor construction, interpolation and application order.
- [x] Checked float axis-angle and direction alignment, typed input errors, extreme-scale tests.
- [x] Document application order and executable local/world movement recipes.
- [x] Explicit +Z forward/+Y up look rotation with parallel-up diagnostics.
- [x] Raw-rotor validation/normalization and checked interpolation.
- [x] Executable orbit position, target aiming and rigid parent/child recipes.
- [x] Compile SDK-facing documentation examples and add an AI usage checklist.
- [ ] CLI end-to-end project validation, DATA generation into obj/, debug/release staging and execution.
- [ ] UV orientation contract and labeled texture regression through import and rendering.

Completed changes are committed and pushed after package tests and boundary checks.
This list does not imply that remaining features are released.

## Component composition

- [x] Independently attachable Position, Rotor and Motor3; runtime resolution and typed diagnostics.
- [x] Position DATA codec and independent component-registry round-trip.
- [x] Initial ECS Collider/PhysicsBody binding and real stepping with pose write-back (root entities, unit scale).
- [ ] IDLE registration, component picker and Inspector integration.

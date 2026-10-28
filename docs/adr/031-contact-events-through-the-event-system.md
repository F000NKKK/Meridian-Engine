# ADR 031: Physics contacts reach audio as events through the shared `EventSystem`

## Status

Accepted. Agreed with team/core and team/physics (no objections; D5 is physics' input).
Builds on [ADR 019](019-ports-in-domain-drivers-as-adapters.md) and
[ADR 025](025-wave-6-runtime-and-composition-root.md).

## Context

Impact sounds need a physics fact (two bodies began touching, how hard, where)
in the audio scene. `docs/dependency-rules.md` forbids `audio-*` from knowing
physics and the reverse; only `runtime-app` may know both. `EventSystem`
already exists as a typed, frame-scoped mailbox with that purpose in mind, but
it is not `Send` (its queues hold `Box<dyn Any>`), so a `Stage` — which is
`Send` — cannot hold a handle to it, and nothing publishes into it.

## Decisions

**D1 — `EventSystem` is `Send` and shared as `Shared<EventSystem>`.** Its
queues hold `Box<dyn Any + Send>`, so published event types must be `Send`.
The API (`publish`, `drain`) does not change. The composition root creates
one `Shared<EventSystem>` and hands a clone to each stage that publishes or
drains. `runtime-app` still owns no application state (ADR 025).

**D2 — the event is a physics fact, defined in `physics-app`.**
`ContactEvent { a, b, point, normal, impact_speed }` carries body indices, the
world-space contact point, the normal from `a` to `b` and the closing speed
along it before the step. It names nothing about sound.

**D3 — contacts are detected beside the stepper, not inside it.**
`ContactWatcher` snapshots body velocities, lets the caller run any
`PhysicsStepper`, then runs the broad and narrow phase on the final poses and
reports pairs that were **not** touching on the previous tick (begin-touch
only; resting contact is silent). Because it reads only `PhysicsSubsystem`,
the sequential and the batched wgpu stepper behave identically and the
`PhysicsStepper` contract is untouched.

**D4 — the physics stage publishes; one new stage voices.** The watcher must
snapshot velocities before the step and read poses after it, so it belongs
inside the stage that runs the step: `PhysicsStepStage::with_contact_events(
watcher, events)` makes it publish `ContactEvent`s into the shared bus after
each step (without it the stage is unchanged). `ContactSoundStage` is new, in
its own file: it drains `ContactEvent`s and, for each, calls an
application-supplied closure `FnMut(&ContactEvent) -> Option<Voice>`; a
returned voice is added to the shared `AudioScene`. Which sound a contact
makes (material, size, speed thresholds) is the application's policy, not the
engine's. Order: physics step -> contact sound -> audio (`add_stage` deps).

**D5 — the watcher is debounced.** A pair counts as separated only after
three consecutive ticks apart (the solver's positional correction can make a
resting pair report no overlap on a tick), and a new touch below the watcher's
`min_impact_speed` is remembered but silent, so it cannot fire later while
the bodies rest. Pairs are keyed `(smaller, larger)` body index, never by
position in the broad phase's output. Raised by team/physics.

## Consequences

- `physics-app` gains `ContactEvent` and `ContactWatcher` (and `gac` is now a
  regular dependency of it); `runtime-app` gains `ContactSoundStage` and the
  edge `runtime-app -> audio-domain` for `Voice`.
- A published event type must be `Send`; events are frame-scoped and a stage
  that never drains a type leaves it queued, so each published type needs
  exactly one drainer.
- Contact events are `f32`-flavoured, like `PhysicsSubsystem`. A `Fixed`
  deterministic simulation would need its own event type; audio output is not
  part of the deterministic simulation (see
  [audio-design.md](../audio-design.md)).
- The watcher costs one extra broad+narrow pass per tick, only for a
  `PhysicsStepStage` built `with_contact_events`. If `PhysicsStepper` later
  exposes the contacts it solved, the watcher can reuse them instead.

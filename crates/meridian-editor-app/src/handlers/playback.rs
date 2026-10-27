//! `Play`, `Pause` and `Step`: the simulation's controls.

use meridian_editor_domain::{Event, PlayState};

use crate::Game;
use crate::outcome::Outcome;

/// `Play`.
pub(crate) fn play(game: &mut Game<'_>) -> Outcome {
    let before = game.simulation.state();
    game.simulation.play();
    changed_since(game, before)
}

/// `Pause`.
pub(crate) fn pause(game: &mut Game<'_>) -> Outcome {
    let before = game.simulation.state();
    game.simulation.pause();
    changed_since(game, before)
}

/// `Step`: one step, paused afterwards.
pub(crate) fn step(game: &mut Game<'_>) -> Outcome {
    let before = game.simulation.state();
    game.simulation.pause();
    game.simulation.step();
    changed_since(game, before)
}

/// `Done`, and `PlayStateChanged` when the state is not `before` any more.
fn changed_since(game: &Game<'_>, before: PlayState) -> Outcome {
    let state = game.simulation.state();
    let outcome = Outcome::done();
    if state == before {
        outcome
    } else {
        outcome.with_event(Event::PlayStateChanged { state })
    }
}

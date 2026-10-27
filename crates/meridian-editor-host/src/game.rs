//! [`Game`]: what a request acts on.

use meridian_ecs_core::World;
use meridian_scene::ComponentRegistry;

use crate::SimulationControl;

/// The running game as a [`crate::Session`] sees it, borrowed for one
/// request: its entities, the components the editor may touch, and the
/// simulation's controls. The host owns all three; a session owns none.
#[derive(Debug)]
pub struct Game<'a> {
    pub(crate) world: &'a mut World,
    pub(crate) registry: &'a ComponentRegistry,
    pub(crate) simulation: &'a mut dyn SimulationControl,
}

impl<'a> Game<'a> {
    /// The game made of `world`, `registry` and `simulation`.
    pub fn new(
        world: &'a mut World,
        registry: &'a ComponentRegistry,
        simulation: &'a mut dyn SimulationControl,
    ) -> Self {
        Game {
            world,
            registry,
            simulation,
        }
    }
}

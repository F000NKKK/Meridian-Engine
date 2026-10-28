//! The component that runs a graph on an entity.

use std::sync::Arc;

use meridian_ecs::Component;

use super::{AnimationGraph, Condition, ParamId, ParamKind, StateId};
use crate::{AnimationPlayer, Repeat, Rig};

/// A parameter's value.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Value {
    Float(f32),
    Bool(bool),
    Trigger(bool),
}

impl Value {
    fn of(kind: ParamKind) -> Self {
        match kind {
            ParamKind::Float => Self::Float(0.0),
            ParamKind::Bool => Self::Bool(false),
            ParamKind::Trigger => Self::Trigger(false),
        }
    }
}

/// Runs an [`AnimationGraph`] for the entity it is attached to, next to its
/// [`Animator`](crate::Animator): gameplay sets parameters, and each frame
/// the animation stage calls [`update`](Self::update) to let a transition
/// fire and steer the animator's player.
#[derive(Debug, Clone)]
pub struct AnimationController {
    graph: Arc<AnimationGraph>,
    state: StateId,
    values: Vec<Value>,
    started: bool,
}

impl Component for AnimationController {}

impl AnimationController {
    /// A controller in the graph's start state, every parameter at zero.
    #[must_use]
    pub fn new(graph: Arc<AnimationGraph>) -> Self {
        let values = graph.params.iter().copied().map(Value::of).collect();
        Self {
            state: graph.start,
            graph,
            values,
            started: false,
        }
    }

    /// The state it is in.
    #[must_use]
    pub fn state(&self) -> StateId {
        self.state
    }

    /// Sets a float parameter; false when `id` is not one.
    pub fn set_float(&mut self, id: ParamId, value: f32) -> bool {
        self.set(id, Value::Float(value))
    }

    /// Sets a bool parameter; false when `id` is not one.
    pub fn set_bool(&mut self, id: ParamId, value: bool) -> bool {
        self.set(id, Value::Bool(value))
    }

    /// Raises a trigger for the next update; false when `id` is not one.
    pub fn trigger(&mut self, id: ParamId) -> bool {
        self.set(id, Value::Trigger(true))
    }

    fn set(&mut self, id: ParamId, value: Value) -> bool {
        match self.values.get_mut(id.0) {
            Some(slot) if std::mem::discriminant(slot) == std::mem::discriminant(&value) => {
                *slot = value;
                true
            }
            _ => false,
        }
    }

    /// Lets at most one transition fire and points `player` at the state it
    /// lands in: the first call puts the player on the start state, and
    /// later ones crossfade it. Triggers are consumed.
    pub fn update(&mut self, player: &mut AnimationPlayer, rig: &Rig) {
        if !self.started {
            self.started = true;
            let start = self.graph.states[self.state.0];
            player.play(start.animation, start.repeat);
        }
        if let Some(index) = self.firing(player, rig) {
            let transition = &self.graph.transitions[index];
            self.state = transition.to;
            let state = self.graph.states[transition.to.0];
            if transition.fade > 0.0 {
                player.crossfade_to(state.animation, transition.fade, state.repeat);
            } else {
                player.play(state.animation, state.repeat);
            }
        }
        for value in &mut self.values {
            if let Value::Trigger(raised) = value {
                *raised = false;
            }
        }
    }

    fn firing(&self, player: &AnimationPlayer, rig: &Rig) -> Option<usize> {
        self.graph.transitions.iter().position(|transition| {
            transition.from.is_none_or(|from| from == self.state)
                && transition
                    .conditions
                    .iter()
                    .all(|condition| self.holds(*condition, player, rig))
        })
    }

    fn holds(&self, condition: Condition, player: &AnimationPlayer, rig: &Rig) -> bool {
        let value = |id: ParamId| self.values.get(id.0).copied();
        match condition {
            Condition::Above(id, limit) => matches!(value(id), Some(Value::Float(v)) if v > limit),
            Condition::Below(id, limit) => matches!(value(id), Some(Value::Float(v)) if v < limit),
            Condition::Is(id, wanted) => value(id) == Some(Value::Bool(wanted)),
            Condition::Triggered(id) => value(id) == Some(Value::Trigger(true)),
            Condition::Finished => {
                let state = self.graph.states[self.state.0];
                state.repeat == Repeat::Once && !player.is_fading() && player.finished(rig)
            }
        }
    }
}

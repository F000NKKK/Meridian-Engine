//! The states of a model and the ways between them.

use super::{
    AnimationState, Condition, GraphError, Motion, ParamId, ParamKind, StateId, Transition,
};
use crate::Repeat;

/// A state machine: states that each play an animation, and transitions
/// that move between them when their conditions hold. Plain data, built once
/// and shared by every controller that runs it.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationGraph {
    pub(super) states: Vec<AnimationState>,
    pub(super) transitions: Vec<Transition>,
    pub(super) params: Vec<ParamKind>,
    pub(super) start: StateId,
}

impl AnimationGraph {
    /// A graph whose first state plays one animation (none is the rest pose),
    /// where a controller begins.
    #[must_use]
    pub fn new(animation: Option<usize>, repeat: Repeat) -> Self {
        let start = AnimationState {
            motion: Motion::Clip(animation),
            repeat,
        };
        Self {
            states: vec![start],
            transitions: Vec::new(),
            params: Vec::new(),
            start: StateId(0),
        }
    }

    /// The state a controller begins in.
    #[must_use]
    pub fn start(&self) -> StateId {
        self.start
    }

    /// Makes `state` the one a controller begins in instead.
    ///
    /// # Errors
    ///
    /// A state that is not this graph's.
    pub fn set_start(&mut self, state: StateId) -> Result<(), GraphError> {
        if state.0 >= self.states.len() {
            return Err(GraphError::UnknownState(state.0));
        }
        self.start = state;
        Ok(())
    }

    /// Adds a state and returns its id.
    ///
    /// # Errors
    ///
    /// A blend that reads a parameter this graph does not have, or one that
    /// is not a float.
    pub fn add_state(&mut self, state: AnimationState) -> Result<StateId, GraphError> {
        for id in state.motion.params() {
            self.check(Condition::Above(id, 0.0))?;
        }
        self.states.push(state);
        Ok(StateId(self.states.len() - 1))
    }

    /// Adds a parameter of `kind` and returns its id.
    pub fn add_param(&mut self, kind: ParamKind) -> ParamId {
        self.params.push(kind);
        ParamId(self.params.len() - 1)
    }

    /// Adds a transition. Transitions leaving one state are tried in the
    /// order they were added, and the first that holds fires.
    ///
    /// # Errors
    ///
    /// A state or parameter that is not this graph's, or a condition on a
    /// parameter of the wrong kind.
    pub fn add_transition(&mut self, transition: Transition) -> Result<(), GraphError> {
        for state in transition.from.iter().chain(Some(&transition.to)) {
            if state.0 >= self.states.len() {
                return Err(GraphError::UnknownState(state.0));
            }
        }
        for condition in &transition.conditions {
            self.check(*condition)?;
        }
        self.transitions.push(transition);
        Ok(())
    }

    fn check(&self, condition: Condition) -> Result<(), GraphError> {
        let Some((id, kind)) = condition.param() else {
            return Ok(());
        };
        match self.params.get(id.0) {
            None => Err(GraphError::UnknownParam(id.0)),
            Some(have) if *have != kind => Err(GraphError::WrongKind(id.0)),
            Some(_) => Ok(()),
        }
    }
}

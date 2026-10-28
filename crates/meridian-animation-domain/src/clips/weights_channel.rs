//! Weight keys aimed at one morph set.

use super::WeightKeys;

/// The animated weights of one morph set.
#[derive(Debug, Clone, PartialEq)]
pub struct WeightsChannel {
    /// Which morph set the keys move: an index the player gives meaning to,
    /// such as the part of a model.
    pub target: usize,
    /// What the weights are and when.
    pub keys: WeightKeys,
}

//! A track aimed at one joint.

use super::Track;
use crate::JointId;

/// One animated property of one joint.
#[derive(Debug, Clone, PartialEq)]
pub struct Channel {
    /// The joint the track moves.
    pub target: JointId,
    /// What changes and when.
    pub track: Track,
}

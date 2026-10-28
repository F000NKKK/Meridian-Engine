//! Nodes that animation moves, as a skeleton of their own (ADR 047).

use std::collections::{BTreeSet, HashMap, HashSet};

use gltf::{Document, Node};
use meridian_asset_domain::{ModelJoint, ModelSkeleton};

use super::skin::Skins;
use crate::limits::MAX_MOTION_JOINTS;

const IDENTITY: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// Where the unskinned mesh nodes that animation moves ended up: each is a
/// joint of one skeleton holding them and their ancestors, and its parts are
/// bound to that joint alone, so the renderer poses them like any skin.
#[derive(Debug, Default)]
pub(crate) struct Motion {
    /// Per mesh node, its `(skeleton, joint)`.
    joints: HashMap<usize, (usize, usize)>,
}

impl Motion {
    /// The `(skeleton, joint)` that moves the mesh at `node`, if animation does.
    pub(crate) fn joint_of(&self, node: usize) -> Option<(usize, usize)> {
        self.joints.get(&node).copied()
    }
}

impl Skins {
    /// Adds a skeleton of the nodes that carry `mesh_nodes` and animation
    /// moves (they, or an ancestor, are the target of a channel), and the
    /// `(skeleton, joint)` of each node in it. Nothing is added when none
    /// moves, or when the skeleton would be larger than a renderer takes.
    pub(crate) fn add_motion(&mut self, document: &Document, mesh_nodes: &[usize]) -> Motion {
        let animated = animated_nodes(document);
        let moving: Vec<usize> = mesh_nodes
            .iter()
            .copied()
            .filter(|&node| self.is_moved(node, &animated))
            .collect();
        let set = self.with_ancestors(&moving);
        if moving.is_empty() || set.len() > MAX_MOTION_JOINTS {
            return Motion::default();
        }
        let nodes: Vec<Node<'_>> = document.nodes().collect();
        let order = self.parents_first(&set);
        let index_of: HashMap<usize, usize> = order
            .iter()
            .enumerate()
            .map(|(joint, &node)| (node, joint))
            .collect();
        let joints = order
            .iter()
            .map(|&node| {
                let (translation, rotation, scale) = nodes[node].transform().decomposed();
                ModelJoint {
                    parent: self
                        .parent_of(node)
                        .and_then(|parent| index_of.get(&parent).copied()),
                    translation,
                    rotation,
                    scale,
                    inverse_bind: IDENTITY,
                }
            })
            .collect();
        let skeleton = self.push_skeleton(ModelSkeleton {
            joints,
            root: IDENTITY,
        });
        for (&node, &joint) in &index_of {
            self.add_node_joint(node, skeleton, joint);
        }
        Motion {
            joints: moving
                .iter()
                .map(|&node| (node, (skeleton, index_of[&node])))
                .collect(),
        }
    }

    /// Whether `node` or an ancestor is in `animated`.
    fn is_moved(&self, node: usize, animated: &HashSet<usize>) -> bool {
        let mut at = Some(node);
        while let Some(current) = at {
            if animated.contains(&current) {
                return true;
            }
            at = self.parent_of(current);
        }
        false
    }

    /// `nodes` and everything above them.
    fn with_ancestors(&self, nodes: &[usize]) -> BTreeSet<usize> {
        let mut set = BTreeSet::new();
        for &node in nodes {
            let mut at = Some(node);
            while let Some(current) = at {
                if !set.insert(current) {
                    break;
                }
                at = self.parent_of(current);
            }
        }
        set
    }

    /// `set` ordered so every node follows its parent: shallow first.
    fn parents_first(&self, set: &BTreeSet<usize>) -> Vec<usize> {
        let depth = |node: usize| {
            let mut depth = 0;
            let mut at = self.parent_of(node);
            while let Some(parent) = at {
                depth += 1;
                at = self.parent_of(parent);
            }
            depth
        };
        let mut order: Vec<usize> = set.iter().copied().collect();
        order.sort_by_key(|&node| (depth(node), node));
        order
    }
}

/// Every node some animation channel targets.
fn animated_nodes(document: &Document) -> HashSet<usize> {
    document
        .animations()
        .flat_map(|animation| animation.channels())
        .map(|channel| channel.target().node().index())
        .collect()
}

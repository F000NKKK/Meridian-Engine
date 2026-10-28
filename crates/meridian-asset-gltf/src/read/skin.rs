//! Skins as skeletons: parents-first joint trees with rest pose and bind.

use gltf::{Document, Node, Skin};
use meridian_asset_domain::{ModelJoint, ModelSkeleton};

use crate::GltfError;

type Matrix = [[f32; 4]; 4];

const IDENTITY: Matrix = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// Where a skin's joints went when they were ordered parents first.
#[derive(Debug, Clone)]
pub(crate) struct SkinMap {
    /// The skeleton's index.
    pub(crate) skeleton: usize,
    /// For each joint in the skin's own order (what `JOINTS_0` indexes), its
    /// index in the skeleton.
    pub(crate) remap: Vec<usize>,
}

/// Every skin of a document, as skeletons and the maps to reach them.
#[derive(Debug, Default)]
pub(crate) struct Skins {
    pub(crate) skeletons: Vec<ModelSkeleton>,
    maps: Vec<SkinMap>,
    /// Per node, the `(skeleton, joint)` pairs it is a joint of.
    node_joints: Vec<Vec<(usize, usize)>>,
    /// The parent node of every node.
    parents: Vec<Option<usize>>,
}

impl Skins {
    /// Reads every skin of `document`. `worlds` holds each reachable node's
    /// accumulated transform by node index, which gives a skeleton the
    /// transform above its roots.
    pub(crate) fn read(
        document: &Document,
        buffers: &[Vec<u8>],
        worlds: &[Option<Matrix>],
    ) -> Result<Self, GltfError> {
        let parents = parents(document);
        let mut skins = Skins {
            node_joints: vec![Vec::new(); document.nodes().len()],
            parents: parents.clone(),
            ..Skins::default()
        };
        for skin in document.skins() {
            let (skeleton, map) = read_skin(&skin, buffers, &parents, worlds)?;
            let index = skins.skeletons.len();
            for (original, node) in skin.joints().enumerate() {
                skins.node_joints[node.index()].push((index, map.remap[original]));
            }
            skins.skeletons.push(skeleton);
            skins.maps.push(SkinMap {
                skeleton: index,
                ..map
            });
        }
        Ok(skins)
    }

    /// The map for `skin`, by the skin's index in the document.
    pub(crate) fn map(&self, skin: &Skin<'_>) -> Option<&SkinMap> {
        self.maps.get(skin.index())
    }

    /// The parent of `node`.
    pub(crate) fn parent_of(&self, node: usize) -> Option<usize> {
        self.parents.get(node).copied().flatten()
    }

    /// Appends `skeleton` and returns its index.
    pub(crate) fn push_skeleton(&mut self, skeleton: ModelSkeleton) -> usize {
        self.skeletons.push(skeleton);
        self.skeletons.len() - 1
    }

    /// Records that `node` is joint `joint` of skeleton `skeleton`.
    pub(crate) fn add_node_joint(&mut self, node: usize, skeleton: usize, joint: usize) {
        self.node_joints[node].push((skeleton, joint));
    }

    /// The `(skeleton, joint)` pairs that `node` is a joint of.
    pub(crate) fn joints_of(&self, node: usize) -> &[(usize, usize)] {
        self.node_joints.get(node).map_or(&[], Vec::as_slice)
    }
}

/// The parent node of every node.
fn parents(document: &Document) -> Vec<Option<usize>> {
    let mut parents = vec![None; document.nodes().len()];
    for node in document.nodes() {
        for child in node.children() {
            parents[child.index()] = Some(node.index());
        }
    }
    parents
}

fn read_skin(
    skin: &Skin<'_>,
    buffers: &[Vec<u8>],
    parents: &[Option<usize>],
    worlds: &[Option<Matrix>],
) -> Result<(ModelSkeleton, SkinMap), GltfError> {
    let nodes: Vec<Node<'_>> = skin.joints().collect();
    let in_skin = |node: usize| nodes.iter().position(|joint| joint.index() == node);
    let skin_parent: Vec<Option<usize>> = nodes
        .iter()
        .map(|joint| nearest_joint(joint.index(), parents, &in_skin))
        .collect();
    let order = parents_first(&skin_parent);
    let mut remap = vec![0; nodes.len()];
    for (sorted, &original) in order.iter().enumerate() {
        remap[original] = sorted;
    }
    let binds = inverse_binds(skin, buffers, nodes.len())?;
    let joints = order
        .iter()
        .map(|&original| {
            let (translation, rotation, scale) = nodes[original].transform().decomposed();
            ModelJoint {
                parent: skin_parent[original].map(|parent| remap[parent]),
                translation,
                rotation,
                scale,
                inverse_bind: binds[original],
            }
        })
        .collect();
    let root = order
        .iter()
        .find(|&&original| skin_parent[original].is_none())
        .and_then(|&original| parents[nodes[original].index()])
        .and_then(|parent| worlds.get(parent).copied().flatten())
        .unwrap_or(IDENTITY);
    Ok((
        ModelSkeleton { joints, root },
        SkinMap { skeleton: 0, remap },
    ))
}

/// The closest ancestor of `node` that `in_skin` places, if any.
fn nearest_joint(
    node: usize,
    parents: &[Option<usize>],
    in_skin: &impl Fn(usize) -> Option<usize>,
) -> Option<usize> {
    let mut above = parents[node];
    while let Some(ancestor) = above {
        if let Some(joint) = in_skin(ancestor) {
            return Some(joint);
        }
        above = parents[ancestor];
    }
    None
}

/// The joint indices ordered so every joint follows its parent: shallow
/// joints first, ties by original position.
fn parents_first(skin_parent: &[Option<usize>]) -> Vec<usize> {
    let depth = |mut joint: usize| {
        let mut depth = 0;
        while let Some(parent) = skin_parent[joint] {
            joint = parent;
            depth += 1;
        }
        depth
    };
    let mut order: Vec<usize> = (0..skin_parent.len()).collect();
    order.sort_by_key(|&joint| (depth(joint), joint));
    order
}

/// One matrix per joint, identity when the skin names none.
fn inverse_binds(
    skin: &Skin<'_>,
    buffers: &[Vec<u8>],
    count: usize,
) -> Result<Vec<Matrix>, GltfError> {
    let reader = skin.reader(|buffer| buffers.get(buffer.index()).map(Vec::as_slice));
    match reader.read_inverse_bind_matrices() {
        None => Ok(vec![IDENTITY; count]),
        Some(matrices) => {
            let matrices: Vec<Matrix> = matrices.collect();
            if matrices.len() < count {
                return Err(GltfError::OutOfBounds("inverse bind matrices"));
            }
            Ok(matrices)
        }
    }
}

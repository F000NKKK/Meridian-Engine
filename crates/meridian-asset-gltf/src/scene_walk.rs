//! Flattening the node hierarchy into world transforms.

use gltf::{Document, Node};
use meridian_gac::mat4_mul;

use crate::GltfError;
use crate::limits::MAX_NODE_DEPTH;

/// A node together with its transform from the model's root.
pub(crate) type PlacedNode<'a> = (Node<'a>, [[f32; 4]; 4]);

/// Every node reachable from the default scene (the first scene when none
/// is marked default; every parentless node when the file has no scenes),
/// each with its accumulated world matrix.
pub(crate) fn flatten(document: &Document) -> Result<Vec<PlacedNode<'_>>, GltfError> {
    let scene = document
        .default_scene()
        .or_else(|| document.scenes().next());
    let roots: Vec<Node<'_>> = match scene {
        Some(scene) => scene.nodes().collect(),
        None => parentless(document),
    };
    let mut placed = Vec::new();
    for root in roots {
        visit(root, IDENTITY, 0, &mut placed)?;
    }
    Ok(placed)
}

/// The accumulated world matrix of each placed node, by node index.
pub(crate) fn worlds(document: &Document, placed: &[PlacedNode<'_>]) -> Vec<Option<[[f32; 4]; 4]>> {
    let mut worlds = vec![None; document.nodes().len()];
    for (node, world) in placed {
        worlds[node.index()] = Some(*world);
    }
    worlds
}

const IDENTITY: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

fn parentless(document: &Document) -> Vec<Node<'_>> {
    let mut is_child = vec![false; document.nodes().len()];
    for node in document.nodes() {
        for child in node.children() {
            is_child[child.index()] = true;
        }
    }
    document
        .nodes()
        .filter(|node| !is_child[node.index()])
        .collect()
}

fn visit<'a>(
    node: Node<'a>,
    parent: [[f32; 4]; 4],
    depth: usize,
    placed: &mut Vec<PlacedNode<'a>>,
) -> Result<(), GltfError> {
    if depth > MAX_NODE_DEPTH {
        return Err(GltfError::TooDeep);
    }
    let world = mat4_mul(parent, node.transform().matrix());
    for child in node.children() {
        visit(child, world, depth + 1, placed)?;
    }
    placed.push((node, world));
    Ok(())
}

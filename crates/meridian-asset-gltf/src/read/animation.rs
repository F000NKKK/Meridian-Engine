//! glTF animations as clips that move joints.

use std::collections::HashMap;

use gltf::animation::util::ReadOutputs;
use gltf::animation::{Interpolation, Property};
use gltf::{Animation, Document};
use meridian_asset_domain::{
    ModelChannel, ModelClip, ModelInterpolation, ModelPart, ModelProperty, ModelWeightsChannel,
};

use crate::GltfError;
use crate::read::skin::Skins;

/// What the animations move: joints of the skeletons, and the morph weights
/// of the parts a node's mesh made.
pub(crate) struct Targets<'a> {
    pub(crate) skins: &'a Skins,
    pub(crate) parts: &'a [ModelPart],
    /// Per node, the indices of the parts its mesh made.
    pub(crate) node_parts: &'a HashMap<usize, Vec<usize>>,
}

/// Every animation of `document` that moves at least one joint or morph
/// weight; other targets are not read.
pub(crate) fn read_all(
    document: &Document,
    buffers: &[Vec<u8>],
    targets: &Targets<'_>,
) -> Result<Vec<ModelClip>, GltfError> {
    let mut clips = Vec::new();
    for animation in document.animations() {
        let channels = read_channels(&animation, buffers, targets.skins)?;
        let weights = read_weights(&animation, buffers, targets)?;
        if !channels.is_empty() || !weights.is_empty() {
            clips.push(ModelClip { channels, weights });
        }
    }
    Ok(clips)
}

/// The morph weight channels of `animation`, one per part the node's mesh made.
fn read_weights(
    animation: &Animation<'_>,
    buffers: &[Vec<u8>],
    targets: &Targets<'_>,
) -> Result<Vec<ModelWeightsChannel>, GltfError> {
    let mut channels = Vec::new();
    for channel in animation.channels() {
        if channel.target().property() != Property::MorphTargetWeights {
            continue;
        }
        let reader = channel.reader(|buffer| buffers.get(buffer.index()).map(Vec::as_slice));
        let times: Vec<f32> = reader
            .read_inputs()
            .ok_or(GltfError::OutOfBounds("animation times"))?
            .collect();
        let Some(ReadOutputs::MorphTargetWeights(values)) = reader.read_outputs() else {
            return Err(GltfError::OutOfBounds("animation values"));
        };
        let values: Vec<f32> = values.into_f32().collect();
        let interpolation = interpolation(channel.sampler().interpolation());
        let parts = targets.node_parts.get(&channel.target().node().index());
        for &part in parts.into_iter().flatten() {
            let count = targets.parts[part]
                .morph
                .as_ref()
                .map_or(0, |m| m.targets.len());
            if count == 0 {
                continue;
            }
            check_weights(&times, &values, count, interpolation)?;
            channels.push(ModelWeightsChannel {
                part,
                interpolation,
                times: times.clone(),
                values: values.clone(),
            });
        }
    }
    Ok(channels)
}

/// One weight per target per key (three sets for a spline), at least one key.
fn check_weights(
    times: &[f32],
    values: &[f32],
    targets: usize,
    interpolation: ModelInterpolation,
) -> Result<(), GltfError> {
    let sets = match interpolation {
        ModelInterpolation::CubicSpline => 3,
        ModelInterpolation::Linear | ModelInterpolation::Step => 1,
    };
    if times.is_empty() || values.len() != times.len() * sets * targets {
        return Err(GltfError::OutOfBounds("animation keys"));
    }
    Ok(())
}

fn read_channels(
    animation: &Animation<'_>,
    buffers: &[Vec<u8>],
    skins: &Skins,
) -> Result<Vec<ModelChannel>, GltfError> {
    let mut channels = Vec::new();
    for channel in animation.channels() {
        let targets = skins.joints_of(channel.target().node().index());
        let Some(property) = property(channel.target().property()) else {
            continue;
        };
        if targets.is_empty() {
            continue;
        }
        let reader = channel.reader(|buffer| buffers.get(buffer.index()).map(Vec::as_slice));
        let times: Vec<f32> = reader
            .read_inputs()
            .ok_or(GltfError::OutOfBounds("animation times"))?
            .collect();
        let values = values(reader.read_outputs())?;
        let interpolation = interpolation(channel.sampler().interpolation());
        check(&times, &values, property, interpolation)?;
        for &(skeleton, joint) in targets {
            channels.push(ModelChannel {
                skeleton,
                joint,
                property,
                interpolation,
                times: times.clone(),
                values: values.clone(),
            });
        }
    }
    Ok(channels)
}

fn property(property: Property) -> Option<ModelProperty> {
    match property {
        Property::Translation => Some(ModelProperty::Translation),
        Property::Rotation => Some(ModelProperty::Rotation),
        Property::Scale => Some(ModelProperty::Scale),
        Property::MorphTargetWeights => None,
    }
}

fn interpolation(interpolation: Interpolation) -> ModelInterpolation {
    match interpolation {
        Interpolation::Linear => ModelInterpolation::Linear,
        Interpolation::Step => ModelInterpolation::Step,
        Interpolation::CubicSpline => ModelInterpolation::CubicSpline,
    }
}

/// The output accessor flattened to numbers.
fn values(outputs: Option<ReadOutputs<'_>>) -> Result<Vec<f32>, GltfError> {
    match outputs.ok_or(GltfError::OutOfBounds("animation values"))? {
        ReadOutputs::Translations(values) | ReadOutputs::Scales(values) => {
            Ok(values.flatten().collect())
        }
        ReadOutputs::Rotations(values) => Ok(values.into_f32().flatten().collect()),
        ReadOutputs::MorphTargetWeights(_) => Ok(Vec::new()),
    }
}

/// One value per key (three for a spline), at least one key.
fn check(
    times: &[f32],
    values: &[f32],
    property: ModelProperty,
    interpolation: ModelInterpolation,
) -> Result<(), GltfError> {
    let per_key = match interpolation {
        ModelInterpolation::CubicSpline => 3 * property.components(),
        ModelInterpolation::Linear | ModelInterpolation::Step => property.components(),
    };
    if times.is_empty() || values.len() != times.len() * per_key {
        return Err(GltfError::OutOfBounds("animation keys"));
    }
    Ok(())
}

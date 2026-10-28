//! glTF animations as clips that move joints.

use gltf::animation::util::ReadOutputs;
use gltf::animation::{Interpolation, Property};
use gltf::{Animation, Document};
use meridian_asset_domain::{ModelChannel, ModelClip, ModelInterpolation, ModelProperty};

use crate::GltfError;
use crate::read::skin::Skins;

/// Every animation of `document` that moves at least one skin joint; other
/// targets (plain nodes, morph weights) are not read.
pub(crate) fn read_all(
    document: &Document,
    buffers: &[Vec<u8>],
    skins: &Skins,
) -> Result<Vec<ModelClip>, GltfError> {
    let mut clips = Vec::new();
    for animation in document.animations() {
        let channels = read_channels(&animation, buffers, skins)?;
        if !channels.is_empty() {
            clips.push(ModelClip { channels });
        }
    }
    Ok(clips)
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

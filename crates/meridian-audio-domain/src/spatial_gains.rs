//! Per-channel gains for one emitter heard by one listener.

use meridian_gac::Vec3;

use crate::{AttenuationModel, Channel, Emitter, Listener, SpeakerLayout};

/// The azimuth of `local_position` (a point already in listener-local
/// space) in the horizontal plane, per the module doc's convention:
/// `0°` = straight ahead (`+X`), positive = toward `+Z` (right).
/// Elevation (`local_position.y`) is ignored — see the module doc.
fn azimuth_of(local_position: Vec3) -> f32 {
    local_position.z.atan2(local_position.x).to_degrees()
}

/// Per-channel gain for one `emitter` as heard by `listener` through
/// `layout`, combining direction ([`SpeakerLayout::pan`](crate::SpeakerLayout::pan)) and distance
/// (`attenuation`). This is what [`Mixer::mix`](crate::Mixer::mix) calls per emitter — exposed
/// standalone so a single emitter/listener pair can be tested in
/// isolation without constructing a [`Mixer`](crate::Mixer).
pub fn spatial_gains(
    listener: &Listener,
    emitter: &Emitter,
    layout: &SpeakerLayout,
    attenuation: &AttenuationModel,
) -> Vec<(Channel, f32)> {
    let mut gains = vec![0.0; layout.speakers.len()];
    spatial_gains_into(listener, *emitter, layout, *attenuation, &mut gains);
    layout
        .speakers
        .iter()
        .zip(gains)
        .map(|(speaker, gain)| (speaker.channel, gain))
        .collect()
}

/// [`spatial_gains`] without allocating: writes the gain of
/// `layout.speakers[i]` into `gains[i]` (see [`SpeakerLayout::pan_into`]).
pub fn spatial_gains_into(
    listener: &Listener,
    emitter: Emitter,
    layout: &SpeakerLayout,
    attenuation: AttenuationModel,
    gains: &mut [f32],
) {
    let local = listener.frame.inverse().transform_point(emitter.position());
    let distance_gain = attenuation.gain(local.length());
    layout.pan_into(azimuth_of(local), gains);
    for gain in gains.iter_mut() {
        *gain *= distance_gain;
    }
}

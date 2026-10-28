//! The stage that turns contact events into voices.

use meridian_audio_app::AudioScene;
use meridian_audio_domain::Voice;
use meridian_physics_app::ContactEvent;

use crate::{EventSystem, Shared, Stage};

/// Built-in stage that drains the [`ContactEvent`]s published this tick and
/// adds a voice to a shared [`AudioScene`] for each one the application's
/// `sound_for` closure answers. Which sound a contact makes (by body,
/// material, impact speed) is the application's policy; this stage only
/// connects physics to audio, which `runtime-app` alone may know about both
/// (ADR 031). Order it after the physics step and before the audio stage.
pub struct ContactSoundStage<F> {
    events: Shared<EventSystem>,
    scene: Shared<AudioScene>,
    sound_for: F,
}

impl<F> ContactSoundStage<F>
where
    F: FnMut(&ContactEvent) -> Option<Voice> + Send,
{
    /// A stage draining `events` into `scene` through `sound_for`.
    #[must_use]
    pub fn new(events: Shared<EventSystem>, scene: Shared<AudioScene>, sound_for: F) -> Self {
        Self {
            events,
            scene,
            sound_for,
        }
    }
}

impl<F> std::fmt::Debug for ContactSoundStage<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContactSoundStage").finish_non_exhaustive()
    }
}

impl<F> Stage for ContactSoundStage<F>
where
    F: FnMut(&ContactEvent) -> Option<Voice> + Send,
{
    fn run(&mut self) {
        let contacts = self.events.lock().drain::<ContactEvent>();
        if contacts.is_empty() {
            return;
        }
        let mut scene = self.scene.lock();
        for contact in &contacts {
            if let Some(voice) = (self.sound_for)(contact) {
                scene.add_voice(voice);
            }
        }
    }
}

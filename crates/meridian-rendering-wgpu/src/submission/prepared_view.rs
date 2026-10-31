//! Camera and lighting resources owned by one prepared view.

use meridian_rendering_domain::{DrawState, Scene3d, ShadowLight, ViewUniforms};

use super::main_pipelines::{MainBindings, PipelineKey, shadow_bind_group};
use super::{LIT_UNIFORM_BYTES, MAX_LIGHTS, SceneRenderer, lit_uniform_bytes};
use crate::{BindGroup, Buffer, BufferUsage, Device, SampledTexture};

/// Uniform and flat-colour bindings that must not change between queued passes.
#[derive(Debug)]
pub(crate) struct PreparedView {
    pub(crate) uniform: Buffer,
    colored: Vec<(PipelineKey, BindGroup)>,
}

impl PreparedView {
    pub(crate) fn new(
        device: &Device,
        renderer: &SceneRenderer,
        scene: &Scene3d,
        shadow: &ShadowLight,
    ) -> Self {
        if scene.lights.len() > MAX_LIGHTS {
            meridian_diagnostics::log_warn!(
                "scene has {} lights, dropping {} beyond the cap of {MAX_LIGHTS}",
                scene.lights.len(),
                scene.lights.len() - MAX_LIGHTS
            );
        }
        let uniform = device.create_buffer(LIT_UNIFORM_BYTES, BufferUsage::Uniform);
        device.write_buffer(
            &uniform,
            &lit_uniform_bytes(
                &ViewUniforms::of_scene(scene, shadow),
                renderer.environment.uniform,
            ),
        );
        let colored = colored_bindings(device, renderer, &uniform);
        Self { uniform, colored }
    }

    pub(crate) fn colored(&self, key: PipelineKey) -> Option<&BindGroup> {
        self.colored
            .iter()
            .find(|(candidate, _)| *candidate == key)
            .map(|(_, group)| group)
    }
}

fn colored_bindings(
    device: &Device,
    renderer: &SceneRenderer,
    uniform: &Buffer,
) -> Vec<(PipelineKey, BindGroup)> {
    let states = [
        DrawState::Opaque,
        DrawState::Transparent,
        DrawState::OnTopOpaque,
        DrawState::OnTopTransparent,
    ];
    let mut groups = Vec::with_capacity(8);
    for state in states {
        for lit in [false, true] {
            let key = PipelineKey {
                textured: false,
                lit,
                state,
            };
            let pipeline = &renderer.pipelines.get(key).pipeline;
            let group = if lit {
                shadow_bind_group(
                    device,
                    pipeline,
                    MainBindings {
                        uniform_buffer: uniform,
                        shadow_map: &renderer.shadow_map,
                        shadow_sampler: &renderer.shadow_sampler,
                        environment: SampledTexture {
                            texture: &renderer.environment.texture,
                            sampler: &renderer.environment.sampler,
                        },
                    },
                )
            } else {
                device.create_uniform_bind_group(pipeline, uniform)
            };
            groups.push((key, group));
        }
    }
    groups
}

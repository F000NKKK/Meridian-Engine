//! Every main-pass pipeline: family × lighting × draw state.

use meridian_rendering_domain::DrawState;

use super::{
    TEXTURED_SHADER_WGSL, UNLIT_SHADER_WGSL, instance_vertex_layout, lit_shader_wgsl,
    lit_textured_shader_wgsl, mesh_vertex_layout,
};
use crate::{
    BindGroup, Buffer, ColorFormat, Device, RenderPipeline, SampledTexture, Sampler, ShaderStages,
    Texture,
};

/// Which pipeline a draw needs.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PipelineKey {
    /// A textured vertex layout (UVs, a bound albedo) rather than flat colour.
    pub textured: bool,
    /// Metallic-roughness shading, rather than `Material::unlit`.
    pub lit: bool,
    /// How the draw composites: blend and depth behaviour.
    pub state: DrawState,
}

/// A main pipeline and, for flat colour draws, the bind group it always
/// uses. Textured draws bind a texture of their own, so theirs is built per
/// draw against `pipeline` (a bind group only fits the pipeline it was made
/// from).
#[derive(Debug)]
pub(crate) struct MainPipeline {
    pub pipeline: RenderPipeline,
    pub shared_bind_group: Option<BindGroup>,
}

/// The shared inputs every main pipeline binds.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MainBindings<'a> {
    pub uniform_buffer: &'a Buffer,
    pub shadow_map: &'a Texture,
    pub shadow_sampler: &'a Sampler,
    pub environment: SampledTexture<'a>,
}

/// The sixteen main pipelines, built once.
#[derive(Debug)]
pub(crate) struct MainPipelines {
    slots: Vec<MainPipeline>,
}

const STATES: [DrawState; 4] = [
    DrawState::Opaque,
    DrawState::Transparent,
    DrawState::OnTopOpaque,
    DrawState::OnTopTransparent,
];

impl MainPipelines {
    /// Compiles each shader once and builds every state's pipeline from it.
    pub(crate) fn new(device: &Device, format: ColorFormat, bindings: MainBindings<'_>) -> Self {
        let shaders = [
            ("meridian-unlit-colored", UNLIT_SHADER_WGSL.to_owned()),
            ("meridian-lit-colored", lit_shader_wgsl()),
            ("meridian-unlit-textured", TEXTURED_SHADER_WGSL.to_owned()),
            ("meridian-lit-textured", lit_textured_shader_wgsl()),
        ]
        .map(|(label, source)| device.create_shader(label, &source));
        let layouts = [mesh_vertex_layout(), instance_vertex_layout()];
        let mut slots = Vec::with_capacity(STATES.len() * shaders.len());
        for state in STATES {
            for (index, shader) in shaders.iter().enumerate() {
                let key = PipelineKey {
                    textured: index >= 2,
                    lit: index % 2 == 1,
                    state,
                };
                let stages = ShaderStages {
                    shader,
                    vertex_entry: "vs_main",
                    fragment_entry: "fs_main",
                };
                let pipeline = device.create_scene_pipeline(&stages, &layouts, format, state);
                slots.push(with_shared_bind_group(device, pipeline, key, bindings));
            }
        }
        Self { slots }
    }

    /// Rebuilds the shared bind groups of the flat lit pipelines against
    /// `bindings`: they hold the environment texture, which changes at runtime.
    pub(crate) fn rebind(&mut self, device: &Device, bindings: MainBindings<'_>) {
        for (index, slot) in self.slots.iter_mut().enumerate() {
            let (textured, lit) = (index % 4 >= 2, index % 2 == 1);
            if lit && !textured {
                slot.shared_bind_group = Some(shadow_bind_group(device, &slot.pipeline, bindings));
            }
        }
    }

    /// The pipeline `key` names.
    pub(crate) fn get(&self, key: PipelineKey) -> &MainPipeline {
        let state = STATES
            .iter()
            .position(|&s| s == key.state)
            .unwrap_or_default();
        &self.slots[state * 4 + usize::from(key.textured) * 2 + usize::from(key.lit)]
    }
}

/// Wraps `pipeline`, binding the shared inputs for flat colour draws.
fn with_shared_bind_group(
    device: &Device,
    pipeline: RenderPipeline,
    key: PipelineKey,
    bindings: MainBindings<'_>,
) -> MainPipeline {
    let shared_bind_group = (!key.textured).then(|| {
        if key.lit {
            shadow_bind_group(device, &pipeline, bindings)
        } else {
            device.create_uniform_bind_group(&pipeline, bindings.uniform_buffer)
        }
    });
    MainPipeline {
        pipeline,
        shared_bind_group,
    }
}

/// The bind group of a flat lit `pipeline`: uniform, shadow map and sampler,
/// environment and sampler.
fn shadow_bind_group(
    device: &Device,
    pipeline: &RenderPipeline,
    bindings: MainBindings<'_>,
) -> BindGroup {
    device.create_shadow_bind_group(
        pipeline,
        bindings.uniform_buffer,
        SampledTexture {
            texture: bindings.shadow_map,
            sampler: bindings.shadow_sampler,
        },
        bindings.environment,
    )
}

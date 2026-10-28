//! Bind group creation on the device.

use crate::{BindGroup, Buffer, Device, LitTextures, SampledTexture, Sampler, Texture};

/// A bind group entry binding `texture`'s view at `binding`.
fn view_entry(binding: u32, texture: &Texture) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: wgpu::BindingResource::TextureView(&texture.view),
    }
}

/// A bind group entry binding `sampler` at `binding`.
fn sampler_entry(binding: u32, sampler: &Sampler) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: wgpu::BindingResource::Sampler(&sampler.raw),
    }
}

impl Device {
    /// Builds a bind group binding `buffer` at `@group(0) @binding(0)` of
    /// `pipeline`'s auto-derived layout (see
    /// [`Device::create_compute_pipeline`](crate::Device::create_compute_pipeline)'s doc comment). Shared by the
    /// compute-dispatch path here and by `rendering-wgpu`'s uniform bind
    /// group for its render pipelines — same single-binding shape either
    /// way.
    pub fn create_single_buffer_bind_group(
        &self,
        pipeline_layout: &wgpu::BindGroupLayout,
        buffer: &Buffer,
    ) -> BindGroup {
        self.create_bind_group(pipeline_layout, &[buffer])
    }

    /// Builds a bind group binding `buffers[i]` at `@group(0)
    /// @binding(i)` of `pipeline_layout`, for pipelines that need more
    /// than one bound resource (e.g. a compute kernel reading from one
    /// storage buffer and writing to another) —
    /// [`Device::create_single_buffer_bind_group`](crate::Device::create_single_buffer_bind_group)'s general form.
    pub fn create_bind_group(
        &self,
        pipeline_layout: &wgpu::BindGroupLayout,
        buffers: &[&Buffer],
    ) -> BindGroup {
        let entries: Vec<wgpu::BindGroupEntry> = buffers
            .iter()
            .enumerate()
            .map(|(i, buffer)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: buffer.raw.as_entire_binding(),
            })
            .collect();
        let raw = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: pipeline_layout,
            entries: &entries,
        });
        BindGroup { raw }
    }

    /// Builds a bind group binding `buffer` at `@group(0) @binding(0)`,
    /// `texture`'s default view at `@binding(1)` and `sampler` at
    /// `@binding(2)` of `pipeline_layout` — the shape a shader that
    /// samples one texture, alongside one uniform buffer, needs. See
    /// [`create_bind_group`](Self::create_bind_group) for the
    /// buffers-only general form this specializes.
    pub fn create_texture_bind_group(
        &self,
        pipeline_layout: &wgpu::BindGroupLayout,
        buffer: &Buffer,
        texture: &Texture,
        sampler: &Sampler,
    ) -> BindGroup {
        let raw = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: pipeline_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.raw.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler.raw),
                },
            ],
        });
        BindGroup { raw }
    }

    /// [`create_texture_bind_group`](Self::create_texture_bind_group)
    /// plus a shadow map at `@binding(3)`, its comparison sampler at
    /// `@binding(4)`, the metallic-roughness, normal, occlusion and
    /// emissive maps at `@binding(5..=8)` and the environment with its
    /// sampler at `@binding(9..=10)` — the shape a textured, shadow-
    /// receiving lit shader needs (uniform, albedo texture, albedo sampler,
    /// shadow map, shadow sampler, the four surface maps, then the
    /// environment).
    pub fn create_textured_shadow_bind_group(
        &self,
        pipeline_layout: &wgpu::BindGroupLayout,
        buffer: &Buffer,
        textures: LitTextures<'_>,
    ) -> BindGroup {
        let LitTextures {
            albedo,
            surface,
            shadow,
            environment,
        } = textures;
        let raw = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: pipeline_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.raw.as_entire_binding(),
                },
                view_entry(1, albedo.texture),
                sampler_entry(2, albedo.sampler),
                view_entry(3, shadow.texture),
                sampler_entry(4, shadow.sampler),
                view_entry(5, surface.metallic_roughness),
                view_entry(6, surface.normal),
                view_entry(7, surface.occlusion),
                view_entry(8, surface.emissive),
                view_entry(9, environment.texture),
                sampler_entry(10, environment.sampler),
            ],
        });
        BindGroup { raw }
    }

    /// Like [`create_textured_shadow_bind_group`](Self::create_textured_shadow_bind_group)
    /// but without an albedo texture/sampler — the shape an *untextured*
    /// (vertex-colored) shadow-receiving lit shader needs: uniform at
    /// `@binding(0)`, shadow map at `@binding(1)`, shadow sampler at
    /// `@binding(2)`, environment at `@binding(3)` and its sampler at
    /// `@binding(4)`.
    pub fn create_shadow_bind_group(
        &self,
        pipeline_layout: &wgpu::BindGroupLayout,
        buffer: &Buffer,
        shadow: SampledTexture<'_>,
        environment: SampledTexture<'_>,
    ) -> BindGroup {
        let raw = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: pipeline_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.raw.as_entire_binding(),
                },
                view_entry(1, shadow.texture),
                sampler_entry(2, shadow.sampler),
                view_entry(3, environment.texture),
                sampler_entry(4, environment.sampler),
            ],
        });
        BindGroup { raw }
    }
}

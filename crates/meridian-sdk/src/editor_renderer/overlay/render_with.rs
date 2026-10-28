//! Drawing the editor's own marks (grid, selection) over the world.

use meridian_ecs::{Entity, World};
use meridian_gac::{Motor3, Scale3};
use meridian_rendering_domain::{
    Material, MaterialHandle, MeshHandle, Renderable3d, Scene3d, extract_scene3d,
};

use super::grid::{LineStyle, grid_lines};
use super::outline::selection_lines;
use super::ribbon::{Line, ribbon_mesh};
use crate::editor_renderer::{EditorCamera, EditorRenderer};

/// What [`render_with`](EditorRenderer::render_with) draws besides the world.
#[derive(Debug, Clone, Copy, Default)]
pub struct ViewOptions<'a> {
    /// The ground grid and its axes (`x` red, `z` blue).
    pub grid: bool,
    /// Entities to draw a box around.
    pub selection: &'a [Entity],
}

impl EditorRenderer {
    /// [`render`](EditorRenderer::render) with the marks `options` asks for.
    pub fn render_with(
        &mut self,
        world: &World,
        camera: &EditorCamera,
        options: &ViewOptions<'_>,
    ) -> Option<&[u8]> {
        let mut scene = extract_scene3d(world, camera.camera(self.aspect()));
        let mut groups = Vec::new();
        if options.grid {
            groups.extend(grid_lines(camera));
        }
        let meshes = &self.session.render.meshes;
        groups.extend(selection_lines(world, meshes, options.selection));
        let drawn = self.add_lines(&mut scene, camera, &groups);
        let pixels_ready = self.render_scene(scene).is_some();
        self.remove_lines(drawn);
        pixels_ready.then_some(self.pixels.as_slice())
    }

    /// One unlit ribbon renderable per style; what it registered is returned
    /// to be taken out again after the frame.
    fn add_lines(
        &mut self,
        scene: &mut Scene3d,
        camera: &EditorCamera,
        groups: &[(LineStyle, Vec<Line>)],
    ) -> Vec<(MeshHandle, MaterialHandle)> {
        let height = f32::from(u16::try_from(self.size.1).unwrap_or(u16::MAX));
        let per_pixel = |distance: f32| 2.0 * distance * (camera.fov_y * 0.5).tan() / height;
        let eye = camera.eye();
        let mut registered = Vec::new();
        for (style, lines) in groups {
            let Some(source) = ribbon_mesh(lines, eye, per_pixel) else {
                continue;
            };
            let render = &mut self.session.render;
            let Ok(mesh) = render.meshes.register(source) else {
                continue;
            };
            let material = render.materials.register(line_material(*style));
            scene.renderables.push(Renderable3d {
                scale: Scale3::UNIT,
                mesh,
                material,
                frame: Motor3::identity(),
                billboard: false,
                casts_shadow: false,
                palette: None,
                morph_weights: Vec::new(),
            });
            registered.push((mesh, material));
        }
        registered
    }

    fn remove_lines(&mut self, drawn: Vec<(MeshHandle, MaterialHandle)>) {
        let render = &mut self.session.render;
        for (mesh, material) in drawn {
            render.meshes.unregister(mesh);
            render.materials.unregister(material);
        }
    }
}

fn line_material(style: LineStyle) -> Material {
    Material {
        base_color_factor: [style.color[0], style.color[1], style.color[2], 1.0],
        unlit: true,
        always_on_top: style.on_top,
        ..Material::default()
    }
}

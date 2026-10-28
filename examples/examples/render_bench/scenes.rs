//! The fixed scenes the benchmark renders.

#![expect(
    clippy::cast_precision_loss,
    reason = "grid indices and frame counts are far below 2^24"
)]

use std::sync::Arc;

use meridian_sdk::assets::{AnimationPlayer, JointPalette, ModelPiece, ModelRig};
use meridian_sdk::{
    Background, EnvironmentMap, FrameRecorder, GraphicsSession, Light, Material, Motor3,
    Renderable3d, Rotor, Scene3d, Sky, Vec3, cube_mesh_source, ground_mesh_source,
    icosphere_mesh_source, look_at_rotor,
};

const SUN: Vec3 = Vec3 {
    x: -0.4,
    y: -1.0,
    z: -0.3,
};

/// The scenes `render_bench` knows, named on its command line.
#[derive(Debug, Clone, Copy)]
pub(crate) enum SceneKind {
    Helmet,
    Shadows,
    Instances,
    Crowd,
}

impl SceneKind {
    pub(crate) const ALL: [Self; 4] = [Self::Helmet, Self::Shadows, Self::Instances, Self::Crowd];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Helmet => "helmet",
            Self::Shadows => "shadows",
            Self::Instances => "instances",
            Self::Crowd => "crowd",
        }
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == text)
    }

    /// The glTF file a scene needs when `--model` is not given.
    fn default_model(self) -> &'static str {
        match self {
            Self::Crowd => "assets/models/CesiumMan.glb",
            _ => "assets/models/DamagedHelmet.glb",
        }
    }
}

/// One animated copy of the crowd's model: its clock and the index of its
/// first renderable.
struct Member {
    player: AnimationPlayer,
    first: usize,
}

struct Crowd {
    rig: Arc<ModelRig>,
    pieces: Vec<ModelPiece>,
    members: Vec<Member>,
}

/// A scene ready to render, with what the camera orbits and what moves.
pub(crate) struct BenchScene {
    pub(crate) scene: Scene3d,
    pub(crate) center: Vec3,
    pub(crate) radius: f32,
    crowd: Option<Crowd>,
    moving_lights: bool,
}

impl BenchScene {
    pub(crate) fn build(
        base: &mut GraphicsSession,
        kind: SceneKind,
        model: Option<&str>,
        animation: usize,
    ) -> Result<Self, String> {
        light_with_sky(base);
        let path = model.unwrap_or_else(|| kind.default_model());
        let mut built = match kind {
            SceneKind::Helmet => model_scene(base, path, (1, animation))?,
            SceneKind::Crowd => model_scene(base, path, (10, animation))?,
            SceneKind::Shadows => shadows_scene(base),
            SceneKind::Instances => instances_scene(base, 64),
        };
        built.scene.background = Background::Environment { blur: 0.0 };
        Ok(built)
    }

    /// Moves everything that moves by `delta` seconds, to scene time `time`.
    pub(crate) fn update(&mut self, delta: f32, time: f32) {
        if let Some(crowd) = &mut self.crowd {
            crowd.advance(&mut self.scene.renderables, delta);
        }
        if self.moving_lights {
            move_lights(&mut self.scene.lights, time);
        }
    }
}

impl Crowd {
    fn advance(&mut self, renderables: &mut [Renderable3d], delta: f32) {
        for member in &mut self.members {
            member.player.advance(&self.rig, delta);
            let palettes: Vec<JointPalette> = self
                .rig
                .matrices_mixed(&member.player.layers())
                .into_iter()
                .map(JointPalette::new)
                .collect();
            for (piece, renderable) in self
                .pieces
                .iter()
                .zip(&mut renderables[member.first..member.first + self.pieces.len()])
            {
                renderable.palette = piece
                    .skeleton
                    .and_then(|skeleton| palettes.get(skeleton).cloned());
            }
        }
    }
}

fn at(position: Vec3) -> Motor3 {
    Motor3::from_rotation_translation(Rotor::identity(), position)
}

fn renderable(
    mesh: meridian_sdk::MeshHandle,
    material: meridian_sdk::MaterialHandle,
    frame: Motor3,
) -> Renderable3d {
    Renderable3d {
        mesh,
        material,
        frame,
        billboard: false,
        casts_shadow: true,
        palette: None,
        morph_weights: Vec::new(),
    }
}

fn sun_light() -> Light {
    Light::Directional {
        direction: Motor3::from_rotation_translation(look_at_rotor(Vec3::ZERO, SUN), Vec3::ZERO),
        color: [1.0, 0.96, 0.9],
        intensity: 1.1,
    }
}

fn light_with_sky(base: &mut GraphicsSession) {
    let sky = Sky {
        sun_direction: [-SUN.x, -SUN.y, -SUN.z],
        sun_radiance: [30.0, 27.0, 22.0],
        sun_radius: 0.06,
        ..Sky::default()
    };
    match EnvironmentMap::from_sky(&sky) {
        Ok(map) => base.render.recorder.set_environment(Some(&map)),
        Err(err) => meridian_sdk::log_error!("sky environment: {err}"),
    }
}

/// `copies * copies` instances of the glTF model at `path` on a grid.
fn model_scene(
    base: &mut GraphicsSession,
    path: &str,
    (copies, animation): (u32, usize),
) -> Result<BenchScene, String> {
    let model = base
        .load_model_gltf(path)
        .map_err(|err| format!("cannot load {path}: {err}"))?;
    let (pieces, rig) = (model.pieces, model.rig);
    let model_radius = model_radius(base, &pieces);
    let spacing = model_radius * 2.2;
    let mut scene = Scene3d {
        lights: vec![sun_light()],
        ambient_ground: [0.12, 0.1, 0.09],
        ambient_sky: [0.22, 0.25, 0.3],
        ..Scene3d::default()
    };
    let materials: Vec<_> = pieces
        .iter()
        .map(|piece| base.render.materials.register(piece.material))
        .collect();
    let animation = (!rig.animations.is_empty()).then(|| animation.min(rig.animations.len() - 1));
    let mut members = Vec::new();
    for index in 0..copies * copies {
        let (column, row) = (index % copies, index / copies);
        let offset = (copies - 1) as f32 * 0.5;
        let position = Vec3::new(
            (column as f32 - offset) * spacing,
            0.0,
            (row as f32 - offset) * spacing,
        );
        members.push(Member {
            player: staggered_player(&rig, animation, index),
            first: scene.renderables.len(),
        });
        for (piece, material) in pieces.iter().zip(&materials) {
            scene
                .renderables
                .push(renderable(piece.mesh, *material, at(position)));
        }
    }
    let radius = model_radius * (copies as f32).max(1.0) * 1.1;
    let crowd = animation.is_some().then_some(Crowd {
        rig,
        pieces,
        members,
    });
    Ok(BenchScene {
        scene,
        center: Vec3::ZERO,
        radius,
        crowd,
        moving_lights: false,
    })
}

/// A player looping animation `animation`, started `index` steps into it so a
/// crowd does not move in lockstep.
fn staggered_player(rig: &ModelRig, animation: Option<usize>, index: u32) -> AnimationPlayer {
    let mut player = AnimationPlayer::new(animation);
    player.advance(rig, index as f32 * 0.13);
    player
}

/// Radius of a sphere around the vertices of `pieces` (bind pose).
fn model_radius(base: &GraphicsSession, pieces: &[ModelPiece]) -> f32 {
    let farthest = pieces
        .iter()
        .filter_map(|piece| base.render.meshes.get(piece.mesh))
        .flat_map(|source| source.positions.iter())
        .map(|p| (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt())
        .fold(0.0_f32, f32::max);
    farthest.max(0.01)
}

fn matte(base: &mut GraphicsSession, color: [f32; 3]) -> meridian_sdk::MaterialHandle {
    base.render.materials.register(Material {
        base_color_factor: [color[0], color[1], color[2], 1.0],
        ..Default::default()
    })
}

/// A floor, forty cubes and spheres, and a sun, a point and two spot lights.
fn shadows_scene(base: &mut GraphicsSession) -> BenchScene {
    let floor = base
        .render
        .meshes
        .register(ground_mesh_source(14.0, 10.0))
        .expect("floor mesh is valid");
    let grey = matte(base, [0.6, 0.6, 0.6]);
    let mut renderables = vec![renderable(floor, grey, Motor3::identity())];
    renderables.extend(props(base));
    let scene = Scene3d {
        renderables,
        lights: std::iter::once(sun_light())
            .chain(moving_lights_at(0.0))
            .collect(),
        ambient_ground: [0.05, 0.05, 0.06],
        ambient_sky: [0.1, 0.12, 0.16],
        ..Scene3d::default()
    };
    BenchScene {
        scene,
        center: Vec3::new(0.0, 1.0, 0.0),
        radius: 12.0,
        crowd: None,
        moving_lights: true,
    }
}

/// Forty cubes and spheres in a grid, at slightly different heights.
fn props(base: &mut GraphicsSession) -> Vec<Renderable3d> {
    let cube = base
        .render
        .meshes
        .register(cube_mesh_source(0.5))
        .expect("cube mesh is valid");
    let sphere = base
        .render
        .meshes
        .register(icosphere_mesh_source(3, 0.6))
        .expect("sphere mesh is valid");
    let (warm, cool) = (matte(base, [0.9, 0.5, 0.3]), matte(base, [0.3, 0.5, 0.9]));
    (0..40_u32)
        .map(|index| {
            let (column, row) = ((index % 8) as f32, (index / 8) as f32);
            let height = 0.5 + (index % 3) as f32 * 0.4;
            let position = Vec3::new(column * 2.4 - 8.4, height, row * 2.4 - 4.8);
            let (mesh, material) = if index % 2 == 0 {
                (cube, warm)
            } else {
                (sphere, cool)
            };
            renderable(mesh, material, at(position))
        })
        .collect()
}

/// The one point and two spot lights of the shadow scene at scene time `time`.
fn moving_lights_at(time: f32) -> Vec<Light> {
    let swing = |phase: f32, radius: f32, height: f32| {
        let angle = time * 0.6 + phase;
        Vec3::new(radius * angle.cos(), height, radius * angle.sin())
    };
    let spot = |phase: f32| {
        let position = swing(phase, 7.0, 6.0);
        Light::Spot {
            position: at(position),
            direction: Motor3::from_rotation_translation(
                look_at_rotor(position, Vec3::ZERO),
                position,
            ),
            color: [1.0, 0.9, 0.7],
            intensity: 6.0,
            range: 18.0,
            inner_angle: 0.3,
            outer_angle: 0.5,
        }
    };
    let point = |phase: f32| Light::Point {
        position: at(swing(phase, 5.0, 2.5)),
        color: [0.6, 0.7, 1.0],
        intensity: 4.0,
        range: 10.0,
    };
    vec![point(0.0), spot(1.5), spot(4.6)]
}

fn move_lights(lights: &mut Vec<Light>, time: f32) {
    lights.truncate(1);
    lights.extend(moving_lights_at(time));
}

/// `side * side` cubes in a grid, sharing one mesh and one material.
fn instances_scene(base: &mut GraphicsSession, side: u32) -> BenchScene {
    let cube = base
        .render
        .meshes
        .register(cube_mesh_source(0.35))
        .expect("cube mesh is valid");
    let material = matte(base, [0.7, 0.7, 0.75]);
    let offset = (side - 1) as f32 * 0.5;
    let renderables = (0..side * side)
        .map(|index| {
            let (column, row) = ((index % side) as f32, (index / side) as f32);
            let height = ((column * 0.7).sin() + (row * 0.5).cos()) * 0.6;
            renderable(
                cube,
                material,
                at(Vec3::new(column - offset, height, row - offset)),
            )
        })
        .collect();
    let scene = Scene3d {
        renderables,
        lights: vec![sun_light()],
        ..Scene3d::default()
    };
    BenchScene {
        scene,
        center: Vec3::ZERO,
        radius: side as f32 * 0.7,
        crowd: None,
        moving_lights: false,
    }
}

//! Model receivers share the textured world's shadow map and fog contract.

use super::{gpu, resource};
use glam::{Mat4, Vec3};
use sindri_gpu::GpuContext;
use sindri_render::{
    ClearOperations, DepthTarget, ExtractedFrame, FogSettings, FrameCamera, FrameCommand,
    FramePass, FrameRenderers, FrameTarget, GlyphRenderer, OffscreenTarget, RenderLayer,
    RenderModel, RenderStage, ShadowSettings, ShapeRenderer, SpriteBatchRenderer, TextRenderer,
    Texture2D, TextureRegistry, TexturedCubeRenderer, Viewport, WorldLighting,
    encode_prepared_frame,
};
use std::sync::Arc;

pub(super) struct Scene {
    pub(super) cubes: TexturedCubeRenderer,
    sprites: SpriteBatchRenderer,
    shapes: ShapeRenderer,
    glyphs: GlyphRenderer,
    text: TextRenderer,
    textures: TextureRegistry,
    frame: sindri_render::PreparedFrame,
}

impl Scene {
    pub(super) fn edit_caster(&mut self, mut edit: impl FnMut(&mut FramePass)) {
        let mut frame = ExtractedFrame::new(self.frame.viewport(), self.frame.clear());
        let last = self.frame.passes().len() - 1;
        for (index, mut pass) in self.frame.passes().iter().cloned().enumerate() {
            if index == last {
                edit(&mut pass);
            }
            frame.push(pass);
        }
        self.frame = frame.prepare().unwrap();
    }

    pub(super) fn new(gpu: &GpuContext) -> Self {
        let format = OffscreenTarget::FORMAT;
        let mut textures = TextureRegistry::new(&gpu.device, &gpu.queue);
        let texture = textures.insert(
            Texture2D::from_rgba8(&gpu.device, &gpu.queue, "caster", 1, 1, &[220, 20, 20, 255])
                .unwrap(),
        );
        let mut data = resource().data().clone();
        for surface in &mut data.surfaces {
            surface.metallic = 0.0;
        }
        let asset: Arc<RenderModel> = Arc::new(RenderModel::new(data).unwrap());
        let eye = Vec3::Z * 5.0;
        let camera = FrameCamera {
            view_projection: glam::camera::rh::proj::directx::orthographic(
                -1.2, 1.2, -0.8, 0.8, 0.1, 10.0,
            ) * glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Y),
            position: eye,
        };
        let mut frame = ExtractedFrame::new(Viewport::new(128, 128), ClearOperations::default());
        for command in [
            FrameCommand::Model {
                model: Mat4::from_translation(Vec3::Z * 0.2),
                asset,
            },
            FrameCommand::TexturedCube {
                model: Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.3),
                    glam::Quat::IDENTITY,
                    Vec3::Z,
                ),
                texture,
            },
        ] {
            frame.push(FramePass::new(
                RenderStage::Opaque3d,
                RenderLayer::WORLD,
                camera,
                command,
            ));
        }
        let mut cubes = TexturedCubeRenderer::new(&gpu.device, format);
        cubes.set_lighting(WorldLighting {
            ambient_intensity: 0.2,
            directional_intensity: 2.0,
            directional_direction: [1.0, 0.0, -2.0],
            ..Default::default()
        });
        Self {
            cubes,
            sprites: SpriteBatchRenderer::new(&gpu.device, format),
            shapes: ShapeRenderer::new(&gpu.device, format),
            glyphs: GlyphRenderer::new(&gpu.device, format),
            text: TextRenderer::new(),
            textures,
            frame: frame.prepare().unwrap(),
        }
    }

    pub(super) fn replace_caster(&mut self, asset: &Arc<RenderModel>, model: Mat4) {
        let mut frame = ExtractedFrame::new(self.frame.viewport(), self.frame.clear());
        for mut pass in self.frame.passes().iter().cloned() {
            if matches!(pass.command, FrameCommand::TexturedCube { .. }) {
                pass.command = FrameCommand::Model {
                    asset: Arc::clone(asset),
                    model,
                };
            }
            frame.push(pass);
        }
        self.frame = frame.prepare().unwrap();
    }

    pub(super) fn set_caster_material(&mut self, material: sindri_render::MaterialFactors) {
        let mut frame = ExtractedFrame::new(self.frame.viewport(), self.frame.clear());
        let last = self.frame.passes().len() - 1;
        for (index, mut pass) in self.frame.passes().iter().cloned().enumerate() {
            if index == last {
                pass.material = Some(material);
            }
            frame.push(pass);
        }
        self.frame = frame.prepare().unwrap();
    }

    fn move_caster(&mut self, x: f32) {
        let mut frame = ExtractedFrame::new(self.frame.viewport(), self.frame.clear());
        for mut pass in self.frame.passes().iter().cloned() {
            if let FrameCommand::TexturedCube { model, .. } = &mut pass.command {
                *model = Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.3),
                    glam::Quat::IDENTITY,
                    Vec3::new(x, 0.0, 1.0),
                );
            }
            frame.push(pass);
        }
        self.frame = frame.prepare().unwrap();
    }

    pub(super) fn draw(
        &mut self,
        gpu: &GpuContext,
        shadows: bool,
        map_size: u32,
        fog: FogSettings,
    ) -> Vec<u8> {
        self.draw_result(gpu, shadows, map_size, fog).unwrap()
    }

    pub(super) fn draw_result(
        &mut self,
        gpu: &GpuContext,
        shadows: bool,
        map_size: u32,
        fog: FogSettings,
    ) -> Result<Vec<u8>, sindri_render::FrameEncodeError> {
        self.cubes.set_shadows(
            &gpu.device,
            ShadowSettings {
                enabled: shadows,
                distance: 2.0,
                map_size,
                bias: 0.001,
            },
        );
        self.cubes.set_fog(fog);
        let target = OffscreenTarget::new(&gpu.device, 128, 128).unwrap();
        let depth = DepthTarget::new(&gpu.device, 128, 128);
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encode_prepared_frame(
            FrameRenderers {
                cube: &mut self.cubes,
                sprites: &mut self.sprites,
                shapes: &mut self.shapes,
                glyphs: &mut self.glyphs,
                text: &mut self.text,
                textures: &self.textures,
            },
            &gpu.device,
            &gpu.queue,
            &mut encoder,
            FrameTarget {
                color: target.view(),
                depth: &depth,
            },
            &self.frame,
        )?;
        let readback = target.copy_to_buffer(&gpu.device, &mut encoder).unwrap();
        gpu.queue.submit([encoder.finish()]);
        Ok(readback.read_rgba8(&gpu.device).unwrap())
    }
}

#[test]
fn model_receives_cube_shadow_after_map_resize_without_geometry_reupload() {
    let Some(gpu) = gpu() else {
        return;
    };
    let mut scene = Scene::new(&gpu);
    let clear = scene.draw(&gpu, false, 256, FogSettings::default());
    for (size, x) in [(256, 0.0), (1024, -0.5), (256, 0.0)] {
        // Move the caster while resizing: stale bindings would sample the last
        // map's old shadow even though all geometry remains resident.
        scene.move_caster(x);
        let shadow = scene.draw(&gpu, true, size, FogSettings::default());
        let darkened = clear
            .chunks_exact(4)
            .zip(shadow.chunks_exact(4))
            .filter(|(a, b)| a[1] > a[0].saturating_add(30) && a[1] > b[1].saturating_add(20))
            .count();
        if size == 1024 {
            assert_eq!(
                darkened, 0,
                "the resized map moves the shadow off the green model"
            );
        } else {
            assert!(
                darkened > 50,
                "green model receives the cube shadow at {size}: {darkened}"
            );
        }
    }
    assert_eq!(
        scene.draw(&gpu, false, 256, FogSettings::default()),
        clear,
        "disabling restores direct light, with no stale shadow binding"
    );
    assert_eq!(scene.cubes.model_cache_stats().uploads, 1);
}

#[test]
fn model_and_cube_share_distance_and_height_fog_and_restore_when_disabled() {
    let Some(gpu) = gpu() else {
        return;
    };
    let mut scene = Scene::new(&gpu);
    let clear = scene.draw(&gpu, false, 256, FogSettings::default());
    for fog in [
        FogSettings {
            enabled: true,
            color: [0.0, 0.0, 1.0],
            start: 0.0,
            distance: 1.0,
            ..Default::default()
        },
        FogSettings {
            enabled: true,
            color: [0.0, 0.0, 1.0],
            start: 100.0,
            height: 2.0,
            height_falloff: 1.0,
            ..Default::default()
        },
        FogSettings {
            enabled: true,
            color: [0.0, 0.0, 1.0],
            start: 0.0,
            distance: 10000.0,
            density: 10.0,
            ..Default::default()
        },
    ] {
        let fogged = scene.draw(&gpu, false, 256, fog);
        for pixel in [(90 * 128 + 90) * 4, (64 * 128 + 64) * 4] {
            assert_eq!(
                &fogged[pixel..pixel + 4],
                &[0, 0, 255, 255],
                "model and caster both use the atmosphere"
            );
        }
    }
    assert_eq!(scene.draw(&gpu, false, 256, FogSettings::default()), clear);
    assert_eq!(scene.cubes.model_cache_stats().uploads, 1);
}

#[test]
fn reflecting_world_caster_preserves_color_and_directional_shadow() {
    let Some(gpu) = gpu() else {
        return;
    };
    let mut scene = Scene::new(&gpu);
    let original = scene.draw(&gpu, true, 256, FogSettings::default());
    let mut frame = ExtractedFrame::new(scene.frame.viewport(), scene.frame.clear());
    for mut pass in scene.frame.passes().iter().cloned() {
        if let FrameCommand::TexturedCube { model, .. } = &mut pass.command {
            *model *= Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        }
        frame.push(pass);
    }
    scene.frame = frame.prepare().unwrap();
    let mirrored = scene.draw(&gpu, true, 256, FogSettings::default());
    assert!(
        original
            .iter()
            .zip(&mirrored)
            .all(|(a, b)| a.abs_diff(*b) <= 2),
        "reflection of a symmetric white cube preserves its exterior and shadow"
    );
    let clear = scene.draw(&gpu, false, 256, FogSettings::default());
    let shadowed = clear
        .chunks_exact(4)
        .zip(mirrored.chunks_exact(4))
        .filter(|(a, b)| a[1] > a[0].saturating_add(30) && a[1] > b[1].saturating_add(20))
        .count();
    assert!(shadowed > 50, "the reflected cube casts a visible shadow");
    assert_eq!(scene.cubes.model_cache_stats().uploads, 1);
}

fn plane_caster(scene: &mut Scene, cached: bool, mirrored: bool, install: bool) {
    let vertices: Vec<_> = [
        [-0.15, -0.15, 0.0],
        [0.15, -0.15, 0.0],
        [0.15, 0.15, 0.0],
        [-0.15, 0.15, 0.0],
    ]
    .into_iter()
    .map(|p| sindri_render::TexturedVertex::new(p, [0.5; 2]).with_normal([0.0, 0.0, 1.0]))
    .collect();
    let model = Mat4::from_scale_rotation_translation(
        Vec3::new(if mirrored { -1.0 } else { 1.0 }, 1.0, 1.0),
        glam::Quat::IDENTITY,
        Vec3::Z,
    );
    let texture = match &scene.frame.passes()[1].command {
        FrameCommand::TexturedCube { texture, .. }
        | FrameCommand::TexturedMesh { texture, .. }
        | FrameCommand::CachedTexturedMesh { texture, .. } => *texture,
        _ => panic!("world caster expected"),
    };
    let command = if cached {
        FrameCommand::CachedTexturedMesh {
            model,
            texture,
            cache: sindri_render::CachedMeshId::new(1),
            revision: 1,
            replacement: install.then(|| {
                sindri_render::CachedTexturedMeshUpload::new(vertices, vec![0, 1, 2, 0, 2, 3])
            }),
            surface: sindri_render::MeshSurface::default(),
        }
    } else {
        FrameCommand::TexturedMesh {
            model,
            texture,
            vertices,
            indices: vec![0, 1, 2, 0, 2, 3],
        }
    };
    let mut frame = ExtractedFrame::new(scene.frame.viewport(), scene.frame.clear());
    for (index, mut pass) in scene.frame.passes().iter().cloned().enumerate() {
        if index == 1 {
            pass.command = command.clone();
        }
        frame.push(pass);
    }
    scene.frame = frame.prepare().unwrap();
}

#[test]
fn reflected_open_surfaces_keep_shadow_winding_and_cached_geometry() {
    let Some(gpu) = gpu() else {
        return;
    };
    for cached in [false, true] {
        let mut scene = Scene::new(&gpu);
        plane_caster(&mut scene, cached, false, true);
        let clear = scene.draw(&gpu, false, 256, FogSettings::default());
        let original = scene.draw(&gpu, true, 256, FogSettings::default());
        let shadowed = clear
            .chunks_exact(4)
            .zip(original.chunks_exact(4))
            .filter(|(a, b)| a[1] > a[0].saturating_add(30) && a[1] > b[1].saturating_add(20))
            .count();
        assert!(shadowed > 50, "an open quad casts before reflection");
        plane_caster(&mut scene, cached, true, false);
        let mirrored = scene.draw(&gpu, true, 256, FogSettings::default());
        assert!(
            original
                .iter()
                .zip(&mirrored)
                .all(|(a, b)| a.abs_diff(*b) <= 2),
            "mirrored quad preserves color and shadow, cached={cached}"
        );
        if cached {
            assert_eq!(scene.cubes.cached_mesh_stats().buffer_uploads, 1);
        }
    }
}

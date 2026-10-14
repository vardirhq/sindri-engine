//! All geometry paths share numeric factors and the same direct GGX response.

use glam::{Mat4, Vec3};
use sindri_gpu::{GpuContext, GpuRequestOptions};
use sindri_render::{
    CachedMeshId, CachedTexturedMeshUpload, ClearOperations, DepthTarget, ExtractedFrame,
    FrameCamera, FrameCommand, FramePass, FrameRenderers, FrameTarget, GlyphRenderer,
    MaterialFactors, MeshSurface, ModelData, ModelGeometry, ModelHierarchyNode, ModelIndexData,
    ModelSurface, ModelVertex, OffscreenTarget, RenderLayer, RenderModel, RenderStage,
    ShapeRenderer, SpriteBatchRenderer, TextRenderer, Texture2D, TextureRegistry,
    TexturedCubeRenderer, TexturedVertex, Viewport, WorldLighting, encode_prepared_frame,
};
use std::sync::Arc;

#[path = "standard_materials/normals.rs"]
mod normals;

struct Scene {
    cube: TexturedCubeRenderer,
    sprites: SpriteBatchRenderer,
    shapes: ShapeRenderer,
    glyphs: GlyphRenderer,
    text: TextRenderer,
    textures: TextureRegistry,
    frames: Vec<sindri_render::PreparedFrame>,
}

impl Scene {
    fn new(gpu: &GpuContext, emissive: [f32; 3]) -> Self {
        let mut textures = TextureRegistry::new(&gpu.device, &gpu.queue);
        let texture = textures.insert(
            Texture2D::from_rgba8(&gpu.device, &gpu.queue, "white", 1, 1, &[255; 4]).unwrap(),
        );
        let positions = [
            [-0.25, -0.25, 0.0],
            [0.25, -0.25, 0.0],
            [0.25, 0.25, 0.0],
            [-0.25, 0.25, 0.0],
        ];
        let vertices: Vec<_> = positions
            .into_iter()
            .map(|p| TexturedVertex::new(p, [0.5; 2]))
            .collect();
        let asset = resource(positions, emissive);
        let eye = Vec3::Z * 5.0;
        let camera = FrameCamera {
            view_projection: glam::camera::rh::proj::directx::orthographic(
                -1.2, 1.2, -0.8, 0.8, 0.1, 10.0,
            ) * glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, Vec3::Y),
            position: eye,
        };
        let mut frames = Vec::new();
        for command in [
            FrameCommand::TexturedCube {
                model: Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.5),
                    glam::Quat::IDENTITY,
                    Vec3::new(-0.6, 0.0, 0.0),
                ),
                texture,
            },
            FrameCommand::TexturedMesh {
                model: Mat4::from_translation(Vec3::new(-0.6, 0.0, 0.25)),
                texture,
                vertices: vertices.clone(),
                indices: vec![0, 1, 2, 0, 2, 3],
            },
            FrameCommand::CachedTexturedMesh {
                model: Mat4::from_translation(Vec3::new(-0.6, 0.0, 0.25)),
                texture,
                cache: CachedMeshId::new(1),
                revision: 1,
                replacement: Some(CachedTexturedMeshUpload::new(
                    vertices.clone(),
                    vec![0, 1, 2, 0, 2, 3],
                )),
                surface: MeshSurface::default(),
            },
        ] {
            let mut frame =
                ExtractedFrame::new(Viewport::new(128, 128), ClearOperations::default());
            for command in [
                command,
                FrameCommand::Model {
                    model: Mat4::from_translation(Vec3::new(0.6, 0.0, 0.25)),
                    asset: Arc::clone(&asset),
                },
            ] {
                frame.push(FramePass::new(
                    RenderStage::Opaque3d,
                    RenderLayer::WORLD,
                    camera,
                    command,
                ));
            }
            frames.push(frame.prepare().unwrap());
        }
        let format = OffscreenTarget::FORMAT;
        Self {
            cube: TexturedCubeRenderer::new(&gpu.device, format),
            sprites: SpriteBatchRenderer::new(&gpu.device, format),
            shapes: ShapeRenderer::new(&gpu.device, format),
            glyphs: GlyphRenderer::new(&gpu.device, format),
            text: TextRenderer::new(),
            textures,
            frames,
        }
    }

    fn draw(
        &mut self,
        gpu: &GpuContext,
        kind: usize,
        material: Option<MaterialFactors>,
        lighting: WorldLighting,
    ) -> Vec<u8> {
        self.cube.set_lighting(lighting);
        let mut frame =
            ExtractedFrame::new(self.frames[kind].viewport(), self.frames[kind].clear());
        for mut pass in self.frames[kind].passes().iter().cloned() {
            pass.material = material;
            frame.push(pass);
        }
        let frame = frame.prepare().unwrap();
        let target = OffscreenTarget::new(&gpu.device, 128, 128).unwrap();
        let depth = DepthTarget::new(&gpu.device, 128, 128);
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encode_prepared_frame(
            FrameRenderers {
                cube: &mut self.cube,
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
            &frame,
        )
        .unwrap();
        let readback = target.copy_to_buffer(&gpu.device, &mut encoder).unwrap();
        gpu.queue.submit([encoder.finish()]);
        readback.read_rgba8(&gpu.device).unwrap()
    }
}

fn pixel(pixels: &[u8], x: usize) -> &[u8] {
    &pixels[(64 * 128 + x) * 4..(64 * 128 + x) * 4 + 4]
}

#[test]
fn cubes_inline_cached_and_imported_geometry_share_standard_factors() {
    let gpu = match pollster::block_on(GpuContext::request(
        &wgpu::Instance::default(),
        None,
        &GpuRequestOptions::default(),
    )) {
        Ok(gpu) => gpu,
        Err(error) => {
            assert!(
                std::env::var_os("SINDRI_REQUIRE_GPU").is_none(),
                "required GPU unavailable: {error}"
            );
            return;
        }
    };
    let mut scene = Scene::new(&gpu, [0.0; 3]);
    let lighting = WorldLighting {
        ambient_intensity: 0.2,
        directional_intensity: 1.0,
        directional_direction: [0.0, 0.0, -1.0],
        ..Default::default()
    };
    let mut swatches = Vec::new();
    for (metallic, roughness) in [(0.0, 1.0), (1.0, 1.0), (1.0, 0.3)] {
        let factors = MaterialFactors {
            base_color: [0.8, 0.3, 0.05, 1.0],
            metallic,
            roughness,
            ..Default::default()
        };
        for kind in 0..3 {
            let image = scene.draw(&gpu, kind, Some(factors), lighting);
            assert!(
                pixel(&image, 32)
                    .iter()
                    .zip(pixel(&image, 95))
                    .all(|(a, b)| a.abs_diff(*b) <= 2),
                "shared GGX for geometry {kind}: {:?} / {:?}",
                pixel(&image, 32),
                pixel(&image, 95)
            );
            if kind == 0 {
                swatches.push(pixel(&image, 32).to_vec());
            }
        }
    }
    assert_ne!(
        swatches[0], swatches[1],
        "metallic factor changes the response"
    );
    assert_ne!(swatches[1], swatches[2], "roughness changes the response");
    check_emission_and_restore(&mut scene, &gpu);
    assert_eq!(scene.cube.model_cache_stats().uploads, 1);
    assert_eq!(scene.cube.cached_mesh_stats().buffer_uploads, 1);
}

fn resource(positions: [[f32; 3]; 4], emissive: [f32; 3]) -> Arc<RenderModel> {
    Arc::new(
        RenderModel::new(ModelData {
            nodes: vec![ModelHierarchyNode {
                name: None,
                local_transform: Mat4::IDENTITY,
                children: vec![],
                mesh: Some(0),
            }],
            roots: vec![0],
            meshes: vec![vec![ModelGeometry {
                vertices: positions
                    .into_iter()
                    .map(|p| ModelVertex {
                        position: p,
                        normal: [0.0, 0.0, 1.0],
                        uv: [0.5; 2],
                    })
                    .collect(),
                indices: ModelIndexData::U16(vec![0, 1, 2, 0, 2, 3]),
                material: Some(0),
            }]],
            surfaces: vec![ModelSurface {
                base_color: [0.0, 1.0, 0.0, 1.0],
                emissive,
                ..Default::default()
            }],
            images: vec![],
        })
        .unwrap(),
    )
}

fn check_emission_and_restore(scene: &mut Scene, gpu: &GpuContext) {
    for kind in 0..3 {
        let image = scene.draw(
            gpu,
            kind,
            Some(MaterialFactors {
                emissive: [0.2, 0.4, 0.1],
                ..Default::default()
            }),
            WorldLighting {
                ambient_intensity: 0.0,
                directional_intensity: 0.0,
                ..Default::default()
            },
        );
        for x in [32, 95] {
            assert!(
                pixel(&image, x)
                    .iter()
                    .zip([124, 170, 89, 255])
                    .all(|(a, b)| a.abs_diff(b) <= 2),
                "self emission: {:?}",
                pixel(&image, x)
            );
        }
        let image = scene.draw(gpu, kind, None, WorldLighting::default());
        assert_eq!(
            pixel(&image, 32),
            &[255; 4],
            "absent mesh override restores legacy appearance"
        );
        assert_eq!(
            pixel(&image, 95),
            &[0, 255, 0, 255],
            "absent model override restores imported factors"
        );
    }
}

#[test]
fn imported_emission_survives_overrides_and_needs_no_world_light() {
    let gpu = match pollster::block_on(GpuContext::request(
        &wgpu::Instance::default(),
        None,
        &GpuRequestOptions::default(),
    )) {
        Ok(gpu) => gpu,
        Err(error) => {
            assert!(
                std::env::var_os("SINDRI_REQUIRE_GPU").is_none(),
                "required GPU unavailable: {error}"
            );
            return;
        }
    };
    let mut scene = Scene::new(&gpu, [0.2, 0.4, 0.1]);
    let lighting = WorldLighting {
        ambient_intensity: 0.0,
        directional_intensity: 0.0,
        ..Default::default()
    };
    let original = scene.draw(&gpu, 0, None, lighting);
    assert!(
        pixel(&original, 95)
            .iter()
            .zip([124, 170, 89, 255])
            .all(|(a, b)| a.abs_diff(b) <= 2)
    );
    assert_eq!(pixel(&original, 32), &[0, 0, 0, 255]);
    let overridden = scene.draw(&gpu, 0, Some(MaterialFactors::default()), lighting);
    assert_eq!(pixel(&overridden, 95), &[0, 0, 0, 255]);
    let restored = scene.draw(&gpu, 0, None, lighting);
    assert_eq!(original, restored);
    assert_eq!(scene.cube.model_cache_stats().uploads, 1);
}

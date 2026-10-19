//! Mesh components, and the pass that draws them.

use sindri_core::World;
use sindri_render::{
    ExtractedFrame, FrameCamera, FrameCommand, FramePass, RenderLayer, RenderStage,
};
use std::collections::BTreeSet;

use crate::{MeshPrimitive, TextureBindings};

use super::camera::ResolvedCameras;
use super::{SceneExtractError, SceneExtractor, transform_matrix};

impl SceneExtractor {
    pub(super) fn push_meshes(
        &self,
        world: &World,
        cameras: &ResolvedCameras,
        textures: &TextureBindings,
        frame: &mut ExtractedFrame,
    ) -> Result<(), SceneExtractError> {
        let mut cache = self.retained_surfaces.borrow_mut();
        let mut live = BTreeSet::new();
        let mut decoded = self.decoded_meshes.borrow_mut();
        // Large mesh payloads must not wait for amortized decode-cache cleanup.
        decoded.retain_active(world);
        for (entity, mesh) in decoded.query(&self.components, world)? {
            let transform = world.world_transform(entity).unwrap_or_default();
            let model = transform_matrix(transform);
            let command = match mesh.primitive {
                MeshPrimitive::Cube => FrameCommand::TexturedCube {
                    model,
                    texture: textures.resolve(&mesh.texture),
                },
                MeshPrimitive::Surface => {
                    let Some((texture, geometry)) = cache.resolve(world, entity, mesh, textures)?
                    else {
                        continue;
                    };
                    live.insert(entity);
                    FrameCommand::RetainedTexturedMesh {
                        model,
                        texture,
                        geometry,
                    }
                }
            };
            let mut pass = FramePass::new(
                RenderStage::Opaque3d,
                RenderLayer(mesh.layer),
                FrameCamera {
                    view_projection: cameras
                        .world
                        .ok_or(SceneExtractError::MissingWorldCamera)?
                        .view_projection,
                    position: cameras
                        .world
                        .ok_or(SceneExtractError::MissingWorldCamera)?
                        .view
                        .inverse()
                        .transform_point3(glam::Vec3::ZERO),
                },
                command,
            );
            if !self.apply_material(world, entity, &mut pass)? {
                continue;
            }
            frame.push(pass);
        }
        cache.retain(&live);
        Ok(())
    }
}

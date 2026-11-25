//! `sindri.text`, laid out in the world.
//!
//! The same instance a HUD label becomes, placed at the entity's world
//! position and drawn through the world camera. Its passes go in after the
//! sprites and shapes of the same layer, so a damage number is over the enemy
//! it names; a higher layer is over it, the way it is for a sprite.

use std::collections::BTreeMap;

use sindri_core::World;
use sindri_render::{
    ExtractedFrame, FrameCamera, FrameCommand, FramePass, RenderLayer, RenderStage, TextInstance,
};

use crate::TextComponent;

use super::camera::ResolvedCameras;
use super::shape::world_model_matrix;
use super::{SceneExtractError, SceneExtractor};

impl SceneExtractor {
    pub(super) fn push_world_text(
        &self,
        world: &World,
        cameras: &ResolvedCameras,
        frame: &mut ExtractedFrame,
    ) -> Result<(), SceneExtractError> {
        let texts = self.components.query::<TextComponent>(world)?;
        if texts.is_empty() {
            return Ok(());
        }
        let camera = cameras.world.ok_or(SceneExtractError::MissingWorldCamera)?;
        // A perspective projection leaves w as the depth, so its last column
        // has no constant term; an orthographic one keeps w at 1.
        let projection = camera.view_projection * camera.view.inverse();
        if projection.w_axis.w.abs() < 1.0e-6 {
            return Self::push_world_text_3d(
                world,
                texts,
                camera.view,
                camera.view_projection,
                frame,
            );
        }
        let mut layers: BTreeMap<i32, Vec<TextInstance>> = BTreeMap::new();
        for (entity, text) in texts {
            if !world.is_active(entity) {
                continue;
            }
            // Where the entity is, ancestors and all. Its scale is never the
            // words': a label keeps the size it was authored at, and a script
            // that wants it bigger sets the size. Its turn is theirs only when
            // the text asks for it; otherwise a label stays level.
            let model = world_model_matrix(world, entity);
            let position = model.w_axis;
            let mut instance = text.instance([position.x, position.y])?;
            if text.follow_rotation {
                // Where the entity's own x axis points, which is its turn in
                // the plane whatever its scale.
                instance = instance.with_rotation(model.x_axis.y.atan2(model.x_axis.x));
            }
            layers.entry(text.layer).or_default().push(instance);
        }
        for (layer, instances) in layers {
            frame.push(FramePass::new(
                RenderStage::Transparent2d,
                RenderLayer(layer),
                FrameCamera {
                    view_projection: camera.view_projection,
                    position: glam::Vec3::ZERO,
                },
                FrameCommand::Text { instances },
            ));
        }
        Ok(())
    }

    /// World text seen through a perspective camera: each label at its
    /// entity's full position in the world, facing the camera, or turned with
    /// its entity when it asks to follow its rotation. Drawn over the 3D scene,
    /// as 2D labels are drawn over their sprites: a wall does not hide it.
    fn push_world_text_3d(
        world: &World,
        texts: Vec<(sindri_core::EntityId, TextComponent)>,
        view: glam::Mat4,
        view_projection: glam::Mat4,
        frame: &mut ExtractedFrame,
    ) -> Result<(), SceneExtractError> {
        let facing = glam::Quat::from_mat4(&view.inverse());
        for (entity, text) in texts {
            if !world.is_active(entity) {
                continue;
            }
            let model = world_model_matrix(world, entity);
            let (_, turn, position) = model.to_scale_rotation_translation();
            let turn = if text.follow_rotation { turn } else { facing };
            let placed = glam::Mat4::from_rotation_translation(turn.normalize(), position);
            frame.push(FramePass::new(
                RenderStage::Transparent2d,
                RenderLayer(text.layer),
                FrameCamera {
                    view_projection: view_projection * placed,
                    position: glam::Vec3::ZERO,
                },
                FrameCommand::Text {
                    instances: vec![text.instance([0.0, 0.0])?],
                },
            ));
        }
        Ok(())
    }
}

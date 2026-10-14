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
        let mut layers: BTreeMap<i32, Vec<TextInstance>> = BTreeMap::new();
        for (entity, text) in texts {
            if !world.is_active(entity) {
                continue;
            }
            // Where the entity is, ancestors and all. Its turn and scale are
            // not the words': a label stays upright and keeps the size it was
            // authored at, and a script that wants it bigger sets the size.
            let position = world_model_matrix(world, entity).w_axis;
            layers
                .entry(text.layer)
                .or_default()
                .push(text.instance([position.x, position.y])?);
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
}

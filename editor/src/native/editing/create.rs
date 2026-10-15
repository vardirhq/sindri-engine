//! Component-bearing creation, committed as a single checked spawn.
use super::super::inspector_panel::draft::{ProjectDefaults, component_default};
use super::{EditorApp, next_game_object_id};
use sindri_core::{CommandBuffer, EntityData, SceneComponent, Transform3D, WorldCommand};

impl EditorApp {
    /// Creates an entity carrying one component at its registered blank, and
    /// selects it.
    ///
    /// A UI image arrives carrying `sindri.ui.image`, which is what puts it in
    /// the UI group and what makes it visible: an empty entity called "UI
    /// something" would be in neither space and would draw nothing. A
    /// directional light arrives aimed down and across the world, so the first
    /// thing it does is light it.
    pub(super) fn create_carrying(&mut self, type_name: &str, name: &str, transform: Transform3D) {
        let Some(payload) = component_default(
            self.scene.components(),
            type_name,
            ProjectDefaults::default(),
        ) else {
            return;
        };
        self.create_component_object(type_name, name, transform, payload);
    }

    fn create_component_object(
        &mut self,
        type_name: &str,
        name: &str,
        transform: Transform3D,
        payload: serde_json::Value,
    ) {
        let entity = self.world.next_handle();
        let mut buffer = CommandBuffer::new();
        buffer.push(WorldCommand::Spawn {
            entity,
            data: Box::new(EntityData {
                source_id: Some(next_game_object_id(&self.world)),
                name: Some(name.to_owned()),
                transform_3d: Some(transform),
                components: [(type_name.to_owned(), payload)].into_iter().collect(),
                ..EntityData::default()
            }),
        });
        self.break_merge_runs();
        if let Err(error) = self.apply_edit(buffer.into_transaction(format!("Create {name}"))) {
            self.report(error.to_string());
            return;
        }
        self.select(Some(entity));
        self.refresh_textures();
    }

    pub(super) fn create_local_light(&mut self, spot: bool) {
        let light = sindri_scene::LightComponent {
            kind: if spot {
                sindri_scene::LightKind::Spot
            } else {
                sindri_scene::LightKind::Point
            },
            intensity: 8.0,
            ..sindri_scene::LightComponent::default()
        };
        self.create_component_object(
            sindri_scene::LightComponent::TYPE_NAME,
            if spot { "Spot Light" } else { "Point Light" },
            Transform3D {
                position: [0.0, 2.0, 0.0],
                rotation: if spot {
                    glam::Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2).to_array()
                } else {
                    glam::Quat::IDENTITY.to_array()
                },
                ..Transform3D::default()
            },
            serde_json::to_value(light).expect("a light serializes"),
        );
    }
}

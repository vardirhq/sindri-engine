//! Particle Lab applies changes to the selected emitter as an undoable edit.
use super::EditorApp;
use eframe::egui;
use sindri_core::{CommandBuffer, EntityId, SceneComponent, Transaction, World, WorldCommand};
use sindri_scene::ParticlesComponent;

impl EditorApp {
    pub(super) fn open_particle_lab(&mut self) {
        let mut lab = sindri_particle_lab::ParticleLab::default();
        if let Some(component) = self
            .selection
            .primary()
            .and_then(|entity| self.world.get(entity))
            .and_then(|data| data.components.get(ParticlesComponent::TYPE_NAME))
            .and_then(|value| serde_json::from_value::<ParticlesComponent>(value.clone()).ok())
        {
            let texture = &component.texture;
            if let Some(bytes) =
                self.file
                    .path()
                    .and_then(|path| path.parent())
                    .and_then(|folder| {
                        let folder = folder.canonicalize().ok()?;
                        let file = folder.join(texture).canonicalize().ok()?;
                        file.starts_with(&folder)
                            .then(|| std::fs::read(file).ok())
                            .flatten()
                    })
            {
                lab.queue_preview_texture(bytes);
            }
            lab.component = component;
        }
        self.particle_lab_target = self.selection.primary();
        self.particle_lab = Some(lab);
    }
    pub(super) fn show_particle_lab(&mut self, context: &egui::Context) {
        let Some(mut lab) = self.particle_lab.take() else {
            return;
        };
        let mut open = true;
        let mut apply = None;
        egui::Window::new("Particle Lab")
            .open(&mut open)
            .default_width(520.0)
            .show(context, |ui| {
                apply = lab.show(ui);
            });
        if let Some(component) = apply {
            self.apply_particle_lab(&component);
        }
        if open {
            self.particle_lab = Some(lab);
        }
    }
    fn apply_particle_lab(&mut self, component: &ParticlesComponent) {
        let transaction = match particle_transaction(
            &self.world,
            self.particle_lab_target,
            self.selection.primary(),
            component,
        ) {
            Ok(transaction) => transaction,
            Err(error) => {
                self.console.error(error);
                return;
            }
        };
        match self.apply_edit(transaction) {
            Ok(()) => self.console.info("Applied particle effect"),
            Err(error) => self.console.error(error.to_string()),
        }
    }
}

fn particle_transaction(
    world: &World,
    target: Option<EntityId>,
    selected: Option<EntityId>,
    component: &ParticlesComponent,
) -> Result<Transaction, String> {
    let entity = target
        .filter(|entity| Some(*entity) == selected)
        .ok_or("Select the original entity with a Particles component")?;
    if !world
        .get(entity)
        .is_some_and(|data| data.components.contains_key(ParticlesComponent::TYPE_NAME))
    {
        return Err("Select an entity with a Particles component".into());
    }
    component.validate().map_err(|error| error.to_string())?;
    if component.texture.trim().is_empty() {
        return Err("Enter a texture asset path".into());
    }
    let payload = serde_json::to_value(component).map_err(|error| error.to_string())?;
    let mut commands = CommandBuffer::new();
    commands.push(WorldCommand::SetComponent {
        entity,
        type_name: ParticlesComponent::TYPE_NAME.to_owned(),
        payload,
    });
    Ok(commands.into_transaction("Apply particle effect"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::{CommandHistory, EntityData};

    #[test]
    fn applying_is_validated_targets_the_original_and_undoes() {
        let mut world = World::default();
        let before = ParticlesComponent {
            texture: "textures/spark.png".into(),
            ..ParticlesComponent::default()
        };
        let entity = world.spawn(EntityData {
            components: [(
                ParticlesComponent::TYPE_NAME.into(),
                serde_json::to_value(&before).unwrap(),
            )]
            .into_iter()
            .collect(),
            ..EntityData::default()
        });
        let other = world.spawn(EntityData::default());
        let changed = ParticlesComponent {
            rate: 70.0,
            ..before.clone()
        };
        assert!(particle_transaction(&world, Some(entity), Some(other), &changed).is_err());
        assert!(particle_transaction(&world, Some(other), Some(other), &changed).is_err());
        let invalid = ParticlesComponent {
            lifetime: 0.0,
            ..before.clone()
        };
        assert!(particle_transaction(&world, Some(entity), Some(entity), &invalid).is_err());
        let transaction =
            particle_transaction(&world, Some(entity), Some(entity), &changed).unwrap();
        let mut history = CommandHistory::default();
        history.apply(transaction, &mut world).unwrap();
        let value = world
            .get(entity)
            .unwrap()
            .components
            .get(ParticlesComponent::TYPE_NAME)
            .unwrap()
            .clone();
        assert_eq!(
            serde_json::from_value::<ParticlesComponent>(value).unwrap(),
            changed
        );
        history.undo(&mut world).unwrap();
        let value = world
            .get(entity)
            .unwrap()
            .components
            .get(ParticlesComponent::TYPE_NAME)
            .unwrap()
            .clone();
        assert_eq!(
            serde_json::from_value::<ParticlesComponent>(value).unwrap(),
            before
        );
    }
}

//! Bringing every instance in a world up to prefabs that changed.
//!
//! The order is what keeps an instance's overrides: each instance is written
//! down as its reference against the prefab it was made from, *before* that
//! prefab is replaced, so what it overrides is the difference from the old
//! prefab. The prefab is then replaced and each reference reconciled with what
//! it now expands to, carrying those differences onto the new prefab.
//!
//! Both editors call it: desktop when the file watcher reads a changed prefab
//! and after *Apply*, the phone when its object workspace saves one.

use sindri_core::{CommandBuffer, EntityId, PrefabDocument, World};

use super::{ScenePrefabs, reconcile};

/// The commands that bring every instance in `world` up to `changed`, each a
/// prefab's asset ID and its new document, keeping what each instance
/// overrides. `prefabs` is left holding the new documents.
///
/// # Errors
/// An instance that cannot be written down, or a prefab that will not read
/// or expand, named.
pub fn follow(
    world: &World,
    prefabs: &mut ScenePrefabs,
    changed: Vec<(String, PrefabDocument)>,
) -> Result<CommandBuffer, String> {
    let mut buffer = CommandBuffer::new();
    follow_into(world, prefabs, changed, None, &mut buffer)?;
    Ok(buffer)
}

/// [`follow`], adding its commands to `buffer`.
///
/// `applied` is the instance whose overrides were just written into its
/// prefab (desktop's *Apply*): what it overrode is now the prefab's, so it
/// starts afresh from the new prefab rather than carrying the same changes
/// onto it a second time.
///
/// # Errors
/// As [`follow`]. On an error `buffer` may hold part of the commands.
pub fn follow_into(
    world: &World,
    prefabs: &mut ScenePrefabs,
    changed: Vec<(String, PrefabDocument)>,
    applied: Option<EntityId>,
    buffer: &mut CommandBuffer,
) -> Result<(), String> {
    let roots: Vec<EntityId> = world
        .entities()
        .filter(|(entity, data)| {
            data.prefab.as_ref().is_some_and(|link| link.root)
                && world.instance_root(*entity) == Some(*entity)
        })
        .map(|(entity, _)| entity)
        .collect();
    let mut placed = Vec::with_capacity(roots.len());
    for root in roots {
        let mut entity = world
            .instance_entity(root, prefabs)
            .map_err(|error| error.to_string())?;
        if applied == Some(root)
            && let Some(instance) = &mut entity.prefab
        {
            instance.overrides.clear();
            instance.removed.clear();
        }
        placed.push((root, entity));
    }
    for (id, document) in changed {
        prefabs.read_placed_by(&document.entities)?;
        prefabs.replace(&id, document);
    }
    let mut rehearsal = world.clone();
    for (root, entity) in &placed {
        let members = world.instance_members(*root);
        reconcile(world, &mut rehearsal, &members, entity, prefabs, buffer)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use sindri_core::{CommandHistory, PrefabDocument, World, WorldCommand};

    use super::{ScenePrefabs, follow, follow_into};
    use crate::prefab::spawn_instance;

    fn lamp(radius: f64, intensity: f64) -> PrefabDocument {
        PrefabDocument::from_json(
            &json!({
                "format_version": 1,
                "metadata": { "name": "Lamp" },
                "entities": [{
                    "id": "lamp",
                    "name": "Lamp",
                    "transform_3d": { "position": [0.0, 0.0, 0.0] },
                    "components": {
                        "sindri.light2d": { "radius": radius, "intensity": intensity }
                    }
                }]
            })
            .to_string(),
        )
        .unwrap()
    }

    /// A lamp instance placed and brightened to 2.5, in a world with its
    /// history.
    fn brightened() -> (ScenePrefabs, World, CommandHistory, sindri_core::EntityId) {
        let mut prefabs = ScenePrefabs::default();
        prefabs.replace("prefabs/lamp.prefab", lamp(4.0, 1.0));
        let mut world = World::default();
        let mut history = CommandHistory::default();
        let mut buffer = sindri_core::CommandBuffer::new();
        let mut rehearsal = world.clone();
        let placed = spawn_instance(
            &mut rehearsal,
            "prefabs/lamp.prefab",
            &prefabs,
            None,
            None,
            &mut buffer,
        )
        .unwrap();
        history
            .apply(buffer.into_transaction("Place"), &mut world)
            .unwrap();
        // This instance is brighter than its prefab.
        let mut brighter = world.get(placed).unwrap().components["sindri.light2d"].clone();
        brighter["intensity"] = json!(2.5);
        let mut buffer = sindri_core::CommandBuffer::new();
        buffer.push(WorldCommand::SetComponent {
            entity: placed,
            type_name: "sindri.light2d".to_owned(),
            payload: brighter,
        });
        history
            .apply(buffer.into_transaction("Brighten"), &mut world)
            .unwrap();
        (prefabs, world, history, placed)
    }

    #[test]
    fn instances_take_a_changed_prefab_and_keep_what_they_override() {
        let (mut prefabs, mut world, mut history, placed) = brightened();
        // The prefab's reach grows, and its own brightness changes too.
        let buffer = follow(
            &world,
            &mut prefabs,
            vec![("prefabs/lamp.prefab".to_owned(), lamp(9.0, 0.5))],
        )
        .unwrap();
        history
            .apply(buffer.into_transaction("Follow"), &mut world)
            .unwrap();
        let light = &world.get(placed).expect("the same entity").components["sindri.light2d"];
        assert_eq!(light["radius"], json!(9.0), "the prefab's change arrives");
        assert_eq!(light["intensity"], json!(2.5), "the override survives");
    }

    #[test]
    fn the_instance_applied_from_starts_afresh_from_its_prefab() {
        let (mut prefabs, mut world, mut history, placed) = brightened();
        // Another instance overrides nothing; Apply writes the first one's
        // brightness, and a reach, into the prefab.
        let mut buffer = sindri_core::CommandBuffer::new();
        let mut rehearsal = world.clone();
        let other = spawn_instance(
            &mut rehearsal,
            "prefabs/lamp.prefab",
            &prefabs,
            None,
            None,
            &mut buffer,
        )
        .unwrap();
        history
            .apply(buffer.into_transaction("Place"), &mut world)
            .unwrap();
        let mut buffer = sindri_core::CommandBuffer::new();
        follow_into(
            &world,
            &mut prefabs,
            vec![("prefabs/lamp.prefab".to_owned(), lamp(6.0, 2.5))],
            Some(placed),
            &mut buffer,
        )
        .unwrap();
        history
            .apply(buffer.into_transaction("Apply"), &mut world)
            .unwrap();
        for lamp in [placed, other] {
            let light = &world.get(lamp).unwrap().components["sindri.light2d"];
            assert_eq!(light["radius"], json!(6.0));
            assert_eq!(light["intensity"], json!(2.5));
        }
        let link = world.get(placed).unwrap().prefab.clone().unwrap();
        let entity = world.instance_entity(placed, &prefabs).unwrap();
        assert!(link.root);
        assert!(
            entity.prefab.unwrap().overrides.is_empty(),
            "what it overrode is the prefab's now"
        );
    }
}

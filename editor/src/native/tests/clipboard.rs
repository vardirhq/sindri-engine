//! Copying entities and pasting them somewhere else.

use sindri_core::{CommandBuffer, CommandHistory, Transform3D, World, WorldCommand};

use super::super::editing::duplicate::copy_under;
use super::super::editing::find_by_source_id;
use super::support::*;

/// What Copy keeps is the world as it was, so a subtree copied and then
/// deleted still pastes whole, at the top level, where it was put.
#[test]
fn a_copy_pastes_whole_after_the_original_is_gone() {
    let mut world = World::from_scene(&nested_scene()).unwrap().world;
    let torso = find_by_source_id(&world, "torso").unwrap();
    let clipboard = world.clone();
    let mut history = CommandHistory::default();
    let mut buffer = CommandBuffer::new();
    buffer.push(WorldCommand::Despawn { entity: torso });
    history
        .apply(buffer.into_transaction("Delete"), &mut world)
        .unwrap();
    assert!(
        find_by_source_id(&world, "arm").is_none(),
        "the arm went too"
    );

    let placed = Transform3D {
        position: [3.0, 4.0, 0.0],
        ..Transform3D::default()
    };
    let mut buffer = CommandBuffer::new();
    let pasted = copy_under(
        &mut world.clone(),
        &clipboard,
        torso,
        None,
        Some(placed),
        &mut buffer,
    )
    .unwrap();
    history
        .apply(buffer.into_transaction("Paste entity"), &mut world)
        .unwrap();
    let copy = world.get(pasted).unwrap();
    assert_eq!(copy.parent, None, "pasted at the top level");
    let position = copy.transform_3d.unwrap().position;
    assert!(
        position
            .iter()
            .zip([3.0, 4.0, 0.0])
            .all(|(got, wanted)| (got - wanted).abs() < 1.0e-6),
        "where it was put, not where it was: {position:?}"
    );
    assert_eq!(copy.children.len(), 1, "with its arm under it");
}

#[test]
fn a_copy_pastes_under_another_entity() {
    let mut world = World::from_scene(&nested_scene()).unwrap().world;
    let torso = find_by_source_id(&world, "torso").unwrap();
    let leg = find_by_source_id(&world, "leg").unwrap();
    let clipboard = world.clone();
    let mut buffer = CommandBuffer::new();
    let pasted = copy_under(
        &mut world.clone(),
        &clipboard,
        torso,
        Some(leg),
        None,
        &mut buffer,
    )
    .unwrap();
    CommandHistory::default()
        .apply(buffer.into_transaction("Paste entity"), &mut world)
        .unwrap();
    assert_eq!(world.get(pasted).unwrap().parent, Some(leg));
    assert!(world.get(leg).unwrap().children.contains(&pasted));
    assert_ne!(
        world.get(pasted).unwrap().source_id,
        world.get(torso).unwrap().source_id,
        "a paste beside its original takes an unused stable ID"
    );
}

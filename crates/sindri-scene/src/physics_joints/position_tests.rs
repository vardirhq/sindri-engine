use super::*;

#[test]
fn checked_position_edits_undo_redo_and_round_trip_keep_fields_and_body_motion() {
    let registry = SceneExtractor::new().unwrap().components().clone();
    let mut world = World::default();
    body(&mut world, "anchor", [0.0, 0.0], RigidBodyKind::Static);
    let moving = body(&mut world, "body", [0.0, 0.0], RigidBodyKind::Dynamic);
    let mut original = registry
        .default_payload(HingeJoint2dComponent::TYPE_NAME)
        .unwrap()
        .clone();
    original["first"] = json!("anchor");
    original["second"] = json!("body");
    original["motor_enabled"] = json!(true);
    original["motor_velocity"] = json!(2.0);
    original["motor_max_torque"] = json!(1.0);
    original["future_field"] = json!(17);
    let owner = world.spawn(EntityData {
        source_id: SceneEntityId::new("hinge").ok(),
        components: [(HingeJoint2dComponent::TYPE_NAME.into(), original.clone())].into(),
        ..EntityData::default()
    });
    let mut physics = ScenePhysics2d::top_down().unwrap();
    for _ in 0..60 {
        physics.step(&mut world, &registry, STEP).unwrap();
    }
    let speed = physics.world().angular_velocity(moving).unwrap();
    let mut edited = original.clone();
    edited["motor_mode"] = json!("position");
    edited["motor_target_angle"] = json!(0.6);
    edited["motor_stiffness"] = json!(5.0);
    edited["motor_damping"] = json!(1.0);
    registry
        .validate_payload(HingeJoint2dComponent::TYPE_NAME, &edited)
        .unwrap();
    let mut commands = CommandBuffer::new();
    commands.push(WorldCommand::SetComponent {
        entity: owner,
        type_name: HingeJoint2dComponent::TYPE_NAME.into(),
        payload: edited.clone(),
    });
    let mut history = CommandHistory::default();
    history
        .apply(commands.into_transaction("Hold hinge angle"), &mut world)
        .unwrap();
    assert!((physics.world().angular_velocity(moving).unwrap() - speed).abs() < 1e-5);
    for _ in 0..600 {
        physics.step(&mut world, &registry, STEP).unwrap();
    }
    assert!((physics.world().pose(moving).unwrap().rotation - 0.6).abs() < 0.03);
    history.undo(&mut world).unwrap();
    assert_eq!(
        world.get(owner).unwrap().components[HingeJoint2dComponent::TYPE_NAME],
        original
    );
    for _ in 0..60 {
        physics.step(&mut world, &registry, STEP).unwrap();
    }
    assert!(physics.world().angular_velocity(moving).unwrap() > 1.5);
    history.redo(&mut world).unwrap();
    let saved = world.to_scene().unwrap();
    assert_eq!(
        saved
            .entities
            .iter()
            .find(|entity| entity.id.as_str() == "hinge")
            .unwrap()
            .components[HingeJoint2dComponent::TYPE_NAME],
        edited
    );
    for _ in 0..600 {
        physics.step(&mut world, &registry, STEP).unwrap();
    }
    assert!((physics.world().pose(moving).unwrap().rotation - 0.6).abs() < 0.03);
    assert_eq!(physics.world().joint_count(), 1);
    let text = saved.to_canonical_json().unwrap();
    let document = sindri_core::SceneDocument::from_json(&text).unwrap();
    let mut reopened = World::from_scene(&document).unwrap().world;
    let moving = reopened
        .entity_for_source_id(&SceneEntityId::new("body").unwrap())
        .unwrap();
    let mut restored_physics = ScenePhysics2d::top_down().unwrap();
    for _ in 0..600 {
        restored_physics
            .step(&mut reopened, &registry, STEP)
            .unwrap();
    }
    assert_eq!(restored_physics.world().joint_count(), 1);
    assert!((restored_physics.world().pose(moving).unwrap().rotation - 0.6).abs() < 0.03);
}

use super::*;

#[test]
fn group_edits_keep_the_body_and_unedited_backend_handles_and_order_by_key() {
    let mut world = PhysicsWorld3d::new([0.0; 3]).unwrap();
    let owner = EntityId::from_bits(1 << 32);
    let pose = PhysicsPose3d::default();
    world
        .insert_static_collider(owner, pose, &[Collider3d::sphere(0.5)])
        .unwrap();
    let original = world.bodies[&owner].colliders[0];
    let body = world.bodies[&owner].body;
    world
        .replace_static_group(owner, 20, pose, &[Collider3d::sphere(0.4)])
        .unwrap();
    let staying = world.bodies[&owner].groups[&20][0];
    world
        .replace_static_group(owner, 10, pose, &[Collider3d::sphere(0.3)])
        .unwrap();
    let earlier = world.bodies[&owner].groups[&10][0];
    assert_eq!(world.bodies[&owner].colliders, [original, earlier, staying]);
    world
        .replace_static_group(owner, 10, pose, &[Collider3d::cuboid([0.5; 3])])
        .unwrap();
    assert_eq!(world.bodies[&owner].body, body);
    assert_eq!(world.bodies[&owner].groups[&20], [staying]);
    assert_eq!(world.bodies[&owner].colliders[0], original);
    assert!(world.backend.colliders.get(earlier).is_none());
    world.remove_static_group(owner, 10).unwrap();
    assert_eq!(world.bodies[&owner].colliders, [original, staying]);
    world.remove(owner);
    assert!(world.backend.colliders.get(original).is_none());
    assert!(world.backend.colliders.get(staying).is_none());
}

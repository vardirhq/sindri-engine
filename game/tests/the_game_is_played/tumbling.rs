//! A block taken back tumbles out of its hole and lands on the world before
//! it is counted: the voxel world is solid to a 3D body, not only to a ray.

use sindri_platform::MouseButton;

use crate::support::*;

fn stock(world: &sindri_core::World, scene: &sindri_scene::SceneExtractor) -> f32 {
    ui_text(world, scene, "Stock")
        .values
        .first()
        .copied()
        .expect("the HUD shows the stock")
}

fn loose(world: &sindri_core::World) -> Option<[f32; 3]> {
    world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some("Loose block"))
        .and_then(|(_, data)| data.transform_3d)
        .map(|transform| transform.position)
}

#[test]
// Cell coordinates near the start are far inside f32's exact integer range.
#[allow(clippy::cast_precision_loss)]
fn a_block_taken_back_lands_on_the_world_before_it_counts() {
    let (mut world, scene, mut session) = session();
    settle(&mut world, &mut session, 2);
    let (ground, at) = in_view(&world, &scene);
    click(&mut world, &mut session, at, MouseButton::Left);
    settle(&mut world, &mut session, 1);
    let laid = stock(&world, &scene);

    hold(&mut world, &mut session, at);
    let start = loose(&world).expect("the block pops out of its cell");
    // One fixed step has already moved it, up and slightly sideways.
    let cell = [ground[0] as f32, ground[2] as f32 + 1.5, ground[1] as f32];
    for axis in 0..3 {
        assert!(
            (start[axis] - cell[axis]).abs() < 0.15,
            "it starts in the cell it filled: {start:?} for {ground:?}"
        );
    }
    // Still on its way: not counted until it lands.
    assert!((stock(&world, &scene) - laid).abs() < f32::EPSILON);

    let mut highest = start[1];
    let mut last = start;
    let mut frames = 0;
    while let Some(at) = loose(&world) {
        highest = highest.max(at[1]);
        last = at;
        frames += 1;
        assert!(frames < 150, "it lands well before the give-up time");
        settle(&mut world, &mut session, 1);
    }
    assert!(highest > start[1] + 0.5, "it popped up: {highest}");
    // It came down onto blocks around where it was, not through them.
    let level = ground[2] as f32;
    assert!(
        last[1] > level - 1.0 && last[1] < level + 2.5,
        "it rests on the ground near its hole: {last:?} over level {level}"
    );
    // The signal reaches the builder, and the builder the HUD, over the next
    // couple of frames.
    settle(&mut world, &mut session, 3);
    assert!(
        (stock(&world, &scene) - (laid + 1.0)).abs() < f32::EPSILON,
        "and is counted back once it lands: {} after {laid}",
        stock(&world, &scene)
    );
}

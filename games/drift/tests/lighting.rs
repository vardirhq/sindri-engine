//! The visible star must agree with the directional light that illuminates Drift.

use drift::Run;
use glam::{Quat, Vec3};

#[test]
fn visible_star_matches_starlight_direction() {
    let game = Run::open().expect("DRIFT opens");
    let star = Vec3::from_array(game.position("local-star")).normalize();
    let sun = game.run.entity("sun").expect("starlight");
    let transform = game.run.world.world_transform(sun).expect("placed");
    let ray = Quat::from_array(transform.rotation).normalize() * Vec3::NEG_Z;
    assert!(star.dot(ray) < -0.999, "the star is opposite the light ray");
    let visual = game.run.entity("local-star").expect("visible star");
    let components = &game.run.world.get(visual).unwrap().components;
    assert!(!components.contains_key("sindri.physics3d.collider"));
}

#[test]
fn helmet_lamp_is_in_the_selected_lights() {
    let mut game = Run::open().expect("DRIFT opens");
    game.wait(0.1);
    let lighting = game.run.scene.lighting(&game.run.world).expect("lighting");
    let helmet = Vec3::from_array(game.position("helmet-lamp"));
    assert!(
        lighting.local_lights.iter().flatten().any(|light| {
            light.spot.is_some() && Vec3::from_array(light.point.position).distance(helmet) < 0.01
        }),
        "the helmet spotlight must reach the renderer"
    );
}

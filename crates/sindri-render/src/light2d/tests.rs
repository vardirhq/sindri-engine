use std::sync::Arc;

use super::*;

fn near(actual: [f32; 3], expected: [f32; 3]) -> bool {
    actual
        .iter()
        .zip(expected)
        .all(|(a, e)| (a - e).abs() < 1.0e-5)
}

fn light(position: [f32; 2]) -> Light2d {
    Light2d {
        position,
        height: 1.0,
        radius: 4.0,
        falloff: 1.0,
        color: [1.0, 0.5, 0.25],
        shadow: None,
    }
}

#[test]
fn light_fades_with_distance_and_stops_at_its_radius() {
    let lighting = Lighting2d {
        ambient: [0.1; 3],
        lights: vec![light([0.0, 0.0])],
    };
    assert!(near(lighting.at([0.0, 0.0]), [1.1, 0.6, 0.35]));
    let half = lighting.at([2.0, 0.0]);
    assert!((half[0] - 0.6).abs() < 1.0e-6, "{half:?}");
    assert!(near(lighting.at([4.0, 0.0]), [0.1; 3]));
}

#[test]
fn a_wall_shadows_what_is_behind_it_and_not_what_is_before_it() {
    let wall = [[2.0, -1.0], [2.0, 1.0]];
    let map = shadow_map_2d([0.0, 0.0], 4.0, &[wall]);
    let mut shadowed = light([0.0, 0.0]);
    shadowed.shadow = Some(Light2dShadow {
        reach: 0.0,
        map: Arc::from(map),
    });
    let lighting = Lighting2d {
        ambient: [0.0; 3],
        lights: vec![shadowed],
    };
    assert!(lighting.at([1.5, 0.0])[0] > 0.0, "before the wall");
    assert!(near(lighting.at([3.0, 0.0]), [0.0; 3]), "behind the wall");
    assert!(lighting.at([-3.0, 0.0])[0] > 0.0, "the other way");
    assert!(lighting.at([3.0, 2.5])[0] > 0.0, "past the wall's end");
}

#[test]
fn the_map_holds_the_nearest_edge_each_way() {
    let map = shadow_map_2d(
        [0.0, 0.0],
        10.0,
        &[[[3.0, -5.0], [3.0, 5.0]], [[5.0, -5.0], [5.0, 5.0]]],
    );
    let east = map[shadow::bin_of([1.0, 0.0])];
    assert!((east - 3.0).abs() < 0.01, "{east}");
    let west = map[shadow::bin_of([-1.0, 0.0])];
    assert!(west > 100.0, "nothing west: {west}");
}

#[test]
fn edges_out_of_reach_are_ignored() {
    let map = shadow_map_2d([0.0, 0.0], 1.0, &[[[3.0, -1.0], [3.0, 1.0]]]);
    assert!(map.iter().all(|distance| *distance > 100.0));
}

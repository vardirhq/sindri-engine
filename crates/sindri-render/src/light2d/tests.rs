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
        source_radius: 0.0,
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

#[test]
fn an_area_light_partially_illuminates_a_shadow_edge() {
    let wall = [[[2.0, -1.0], [2.0, 1.0]]];
    let mut lamp = light([0.0, 0.0]);
    lamp.shadow = Some(Light2dShadow {
        reach: 0.0,
        source_radius: 0.5,
        map: Arc::from(shadow_maps_2d(lamp.position, lamp.radius, 0.5, &wall)),
    });
    let lit = Lighting2d {
        ambient: [0.0; 3],
        lights: vec![lamp],
    };
    let edge = lit.at([3.0, 1.5])[0];
    let unshadowed = (1.0_f32 - 3.0_f32.hypot(1.5) / 4.0).max(0.0);
    assert!(
        edge > 0.0 && edge < unshadowed,
        "partial shadow: {edge}, open: {unshadowed}"
    );
    assert!(lit.at([1.5, 0.0])[0] > 0.0, "the light side stays lit");
    assert!(
        near(lit.at([3.0, 0.0]), [0.0; 3]),
        "behind the wall stays dark"
    );
}

#[test]
fn zero_area_preserves_one_point_shadow_map() {
    let edges = [[[1.0, -1.0], [1.0, 1.0]]];
    let point = shadow_map_2d([0.0, 0.0], 4.0, &edges);
    let area = shadow_maps_2d([0.0, 0.0], 4.0, 0.0, &edges);
    assert_eq!(point.len(), area.len());
    assert!(
        point
            .iter()
            .zip(area)
            .all(|(a, b)| a.to_bits() == b.to_bits())
    );
}

#[test]
fn soft_shadows_follow_moving_lights_and_occluders() {
    let origin = [0.0, 0.0];
    let point = [3.0, 0.0];
    let visibility = |position, wall| {
        let mut lamp = light(position);
        lamp.radius = 8.0;
        lamp.shadow = Some(Light2dShadow {
            reach: 0.0,
            source_radius: 0.5,
            map: Arc::from(shadow_maps_2d(position, 8.0, 0.5, &[wall])),
        });
        Lighting2d {
            ambient: [0.0; 3],
            lights: vec![lamp],
        }
        .at(point)[0]
    };
    let wall = [[2.0, -2.0], [2.0, 2.0]];
    assert!(
        visibility(origin, wall) < 0.001,
        "the wall blocks the light"
    );
    assert!(
        visibility([4.0, 0.0], wall) > 0.1,
        "moving the light clears the shadow"
    );
    assert!(
        visibility(origin, [[4.0, -2.0], [4.0, 2.0]]) > 0.1,
        "moving the wall clears the shadow"
    );
}

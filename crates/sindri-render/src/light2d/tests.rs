use std::sync::Arc;

use super::*;
use crate::light2d::{
    LightCone2d, SHADOW_INTERVAL_FLOATS_2D, SHADOW_INTERVALS_2D, SHADOW_MAP_LEN_2D, sees,
};

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
        cone: None,
    }
}

#[test]
fn light_fades_with_distance_and_stops_at_its_radius() {
    let lighting = Lighting2d {
        ambient: [0.1; 3],
        sight: None,
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
        sight: None,
        lights: vec![shadowed],
    };
    assert!(lighting.at([1.5, 0.0])[0] > 0.0, "before the wall");
    assert!(near(lighting.at([3.0, 0.0]), [0.0; 3]), "behind the wall");
    assert!(lighting.at([-3.0, 0.0])[0] > 0.0, "the other way");
    assert!(lighting.at([3.0, 2.5])[0] > 0.0, "past the wall's end");
}

/// The intervals a map really uses: those before the padding.
fn used(map: &[f32]) -> usize {
    map.chunks(SHADOW_INTERVAL_FLOATS_2D)
        .take_while(|interval| interval[0] < 1.0e8)
        .count()
}

fn blocked(map: &[f32], point: [f32; 2]) -> bool {
    !sees(map, point, 0.0)
}

#[test]
fn the_nearest_edge_each_way_is_the_one_that_blocks() {
    let map = shadow_map_2d(
        [0.0, 0.0],
        10.0,
        &[[[3.0, -5.0], [3.0, 5.0]], [[5.0, -5.0], [5.0, 5.0]]],
    );
    assert!(!blocked(&map, [2.9, 0.0]), "before the near wall");
    assert!(blocked(&map, [3.1, 0.0]), "behind the near wall");
    assert!(blocked(&map, [6.0, 0.0]), "and so behind the far one");
    assert!(!blocked(&map, [-8.0, 0.0]), "nothing west");
}

/// The boundary of a shadow is where the edge's end is, not within a bin of it.
#[test]
fn a_shadow_ends_exactly_where_its_edge_does() {
    let map = shadow_map_2d([0.0, 0.0], 8.0, &[[[2.0, -1.0], [2.0, 1.0]]]);
    // The ray through the wall's top end, (2, 1), passes (4, 2).
    assert!(blocked(&map, [4.0, 1.99]), "just under that ray is shadow");
    assert!(!blocked(&map, [4.0, 2.01]), "just over it is light");
    assert!(blocked(&map, [6.0, 2.98]), "and the same further out");
    assert!(!blocked(&map, [6.0, 3.02]));
    assert!(blocked(&map, [4.0, -1.99]), "and on the other side");
    assert!(!blocked(&map, [4.0, -2.01]));
}

/// Three intervals describe one wall: open, blocked, open. The map holds no
/// more than that, so it costs what the scene has, not a fixed table.
#[test]
fn one_wall_is_three_intervals() {
    let map = shadow_map_2d([0.0, 0.0], 8.0, &[[[2.0, -1.0], [2.0, 1.0]]]);
    assert_eq!(used(&map), 3);
    assert_eq!(map.len(), SHADOW_MAP_LEN_2D);
    assert!(map[3] < 0.0, "open before the wall");
    assert!(map[SHADOW_INTERVAL_FLOATS_2D + 3] > 0.0, "the wall");
    assert!(map[2 * SHADOW_INTERVAL_FLOATS_2D + 3] < 0.0, "open after");
}

/// A corner is a corner: two walls meeting shadow the wedge behind them and
/// leave both sides of it lit.
#[test]
fn two_walls_meeting_cast_a_corner() {
    let map = shadow_map_2d(
        [0.0, 0.0],
        10.0,
        &[[[2.0, 2.0], [2.0, -2.0]], [[2.0, 2.0], [-2.0, 2.0]]],
    );
    assert!(blocked(&map, [4.0, 4.0]), "behind the corner");
    assert!(blocked(&map, [4.0, 0.5]), "behind the east wall");
    assert!(blocked(&map, [0.5, 4.0]), "behind the north wall");
    assert!(!blocked(&map, [3.0, -4.0]), "past the east wall's far end");
    assert!(!blocked(&map, [-4.0, 3.0]), "past the north wall's far end");
    assert!(!blocked(&map, [1.5, 1.5]), "in front of the corner");
}

#[test]
fn reach_lets_light_past_an_edge_a_little_way() {
    let map = shadow_map_2d([0.0, 0.0], 8.0, &[[[2.0, -1.0], [2.0, 1.0]]]);
    assert!(
        sees(&map, [2.3, 0.0], 0.5),
        "within reach of the wall's face"
    );
    assert!(!sees(&map, [2.3, 0.0], 0.0), "without reach it is dark");
    assert!(!sees(&map, [3.0, 0.0], 0.5), "and well behind it still is");
}

#[test]
fn a_light_standing_on_an_edge_line_is_not_confused() {
    // The wall's line runs through the light: no ray crosses it, so it blocks nothing.
    let map = shadow_map_2d([0.0, 0.0], 8.0, &[[[1.0, 0.0], [3.0, 0.0]]]);
    assert_eq!(used(&map), 1);
    assert!(map.iter().all(|value| value.is_finite()));
}

#[test]
fn more_edges_than_intervals_still_block_the_wide_ones() {
    // A ring of hundreds of tiny walls far out, and one wide wall near.
    let mut edges = vec![[[2.0, -3.0], [2.0, 3.0]]];
    for step in 0..600_u16 {
        let angle = f32::from(step) / 600.0 * std::f32::consts::TAU;
        let (a, b) = (angle, angle + 0.004);
        edges.push([
            [7.0 * a.cos(), 7.0 * a.sin()],
            [7.0 * b.cos(), 7.0 * b.sin()],
        ]);
    }
    let map = shadow_map_2d([0.0, 0.0], 9.0, &edges);
    assert_eq!(map.len(), SHADOW_MAP_LEN_2D);
    assert!(used(&map) <= SHADOW_INTERVALS_2D);
    assert!(map.iter().all(|value| value.is_finite()));
    assert!(blocked(&map, [3.0, 0.0]), "the wide wall still blocks");
    assert!(!blocked(&map, [1.0, 0.0]));
}

#[test]
fn edges_out_of_reach_are_ignored() {
    let map = shadow_map_2d([0.0, 0.0], 1.0, &[[[3.0, -1.0], [3.0, 1.0]]]);
    assert_eq!(used(&map), 1);
    assert!(!blocked(&map, [0.9, 0.0]));
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
        sight: None,
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
    assert_eq!(point.len(), SHADOW_MAP_LEN_2D);
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
            sight: None,
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

#[test]
fn a_spot_light_lights_its_cone_and_not_what_is_behind_it() {
    let mut beam = light([0.0, 0.0]);
    beam.cone = Some(LightCone2d::new(0.0, 60.0, 0.0));
    let lighting = Lighting2d {
        ambient: [0.0; 3],
        sight: None,
        lights: vec![beam],
    };
    assert!(lighting.at([2.0, 0.0])[0] > 0.4, "ahead of it");
    assert!(
        near(lighting.at([-2.0, 0.0]), [0.0; 3]),
        "behind it nothing is lit"
    );
    assert!(
        near(lighting.at([0.0, 2.0]), [0.0; 3]),
        "square across it, outside a 60 degree beam"
    );
    // 20 degrees off the middle is inside a beam 30 degrees to a side.
    let inside = [
        2.0 * 20.0_f32.to_radians().cos(),
        2.0 * 20.0_f32.to_radians().sin(),
    ];
    assert!(lighting.at(inside)[0] > 0.4, "inside the cone");
    // 40 degrees is outside it.
    let outside = [
        2.0 * 40.0_f32.to_radians().cos(),
        2.0 * 40.0_f32.to_radians().sin(),
    ];
    assert!(near(lighting.at(outside), [0.0; 3]), "outside the cone");
}

#[test]
fn the_edge_of_a_soft_beam_fades_and_a_hard_one_does_not() {
    let hard = LightCone2d::new(0.0, 60.0, 0.0);
    let soft = LightCone2d::new(0.0, 60.0, 1.0);
    let at = |degrees: f32| [degrees.to_radians().cos(), degrees.to_radians().sin()];
    assert!(
        (hard.factor(at(25.0)) - 1.0).abs() < 1.0e-3,
        "inside a hard edge"
    );
    assert!(hard.factor(at(35.0)) < 1.0e-3, "outside it");
    let (near_middle, near_edge) = (soft.factor(at(10.0)), soft.factor(at(25.0)));
    assert!(
        near_edge > 0.0 && near_edge < near_middle && near_middle < 1.0,
        "a soft beam fades toward its edge: {near_middle}, {near_edge}"
    );
    assert!(
        (soft.factor(at(0.0)) - 1.0).abs() < 1.0e-6,
        "bright along the middle"
    );
    assert!(
        (soft.factor([0.0, 0.0]) - 1.0).abs() < 1.0e-6,
        "and at the light itself"
    );
}

fn viewer(radius: f32, unseen: f32, edges: &[[[f32; 2]; 2]]) -> Sight2d {
    Sight2d {
        position: [0.0, 0.0],
        radius,
        unseen,
        reach: 0.0,
        map: Arc::from(shadow_map_2d([0.0, 0.0], 100.0, edges)),
    }
}

#[test]
fn a_viewer_dims_what_a_wall_hides_and_nothing_else() {
    let sight = viewer(0.0, 0.25, &[[[2.0, -1.0], [2.0, 1.0]]]);
    assert!(
        (sight.factor([1.0, 0.0]) - 1.0).abs() < 1.0e-6,
        "before the wall"
    );
    assert!(
        (sight.factor([4.0, 0.0]) - 0.25).abs() < 1.0e-6,
        "behind it"
    );
    assert!(
        (sight.factor([-4.0, 0.0]) - 1.0).abs() < 1.0e-6,
        "the other way"
    );
    assert!(
        (sight.factor([4.0, 3.0]) - 1.0).abs() < 1.0e-6,
        "past its end"
    );
}

#[test]
fn a_viewer_with_a_radius_fades_out_at_its_edge() {
    let sight = viewer(8.0, 0.2, &[]);
    assert!(
        (sight.factor([4.0, 0.0]) - 1.0).abs() < 1.0e-6,
        "well inside"
    );
    let edge = sight.factor([7.0, 0.0]);
    assert!(edge > 0.2 && edge < 1.0, "fading near the edge: {edge}");
    assert!((sight.factor([9.0, 0.0]) - 0.2).abs() < 1.0e-6, "beyond it");
}

#[test]
fn what_is_out_of_sight_is_dimmed_in_the_light_that_reaches_it() {
    let lamp = Lighting2d {
        ambient: [0.0; 3],
        lights: vec![light([3.0, 0.0])],
        sight: Some(viewer(0.0, 0.5, &[[[2.0, -1.0], [2.0, 1.0]]])),
    };
    let unseen = lamp.at([4.0, 0.0])[0];
    let open = Lighting2d {
        sight: None,
        ..lamp.clone()
    }
    .at([4.0, 0.0])[0];
    assert!(
        open > 0.0 && (unseen - open * 0.5).abs() < 1.0e-5,
        "{unseen} against {open}"
    );
}

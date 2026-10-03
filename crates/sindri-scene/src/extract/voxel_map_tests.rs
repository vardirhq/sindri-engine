use glam::{Mat4, Vec3};

use super::*;

fn above(x: f32, y: f32, half: f32, roll: f32) -> ResolvedCamera {
    let up = Vec3::new(-roll.sin(), roll.cos(), 0.0);
    let view =
        glam::camera::rh::view::look_at_mat4(Vec3::new(x, y, 10.0), Vec3::new(x, y, 0.0), up);
    let projection =
        glam::camera::rh::proj::directx::orthographic(-half, half, -half, half, 0.1, 100.0);
    ResolvedCamera {
        view,
        view_projection: projection * view,
        framed_half_height: half,
    }
}

#[test]
fn the_columns_in_view_are_those_under_the_camera_padded_by_one() {
    let (min, max) = visible_columns(Mat4::IDENTITY, above(10.0, -20.0, 4.0, 0.0)).unwrap();
    // X from 6 to 14 and Y from -24 to -16, which is rows 16 to 24.
    assert_eq!(min, [5, 15]);
    assert_eq!(max, [15, 25]);
}

#[test]
fn a_turned_camera_sees_the_corners_it_swings_over() {
    let (min, max) = visible_columns(
        Mat4::IDENTITY,
        above(0.0, 0.0, 4.0, std::f32::consts::FRAC_PI_4),
    )
    .unwrap();
    // A square half four across turned an eighth reaches about 5.66 out.
    assert_eq!(min, [-7, -7]);
    assert_eq!(max, [6, 6]);
}

#[test]
fn the_map_moves_with_its_entity() {
    let shifted = Mat4::from_translation(Vec3::new(100.0, 0.0, 0.0));
    let (min, max) = visible_columns(shifted, above(100.0, 0.0, 2.0, 0.0)).unwrap();
    assert_eq!(min, [-3, -3]);
    assert_eq!(max, [3, 3]);
}

#[test]
fn a_camera_pulled_far_back_sees_a_bounded_map() {
    let (min, max) = visible_columns(Mat4::IDENTITY, above(0.0, 0.0, 100_000.0, 0.0)).unwrap();
    for axis in 0..2 {
        assert!(max[axis] - min[axis] <= MOST_CHUNKS_ACROSS * CHUNK);
    }
}

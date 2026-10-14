use std::path::Path;

use sindri_assets::normal_map::{FLAT_NORMAL, NormalMapPreset, encode_png};

use super::*;

/// Writes a `width` by `height` gray ramp, rising to the right, as a PNG.
fn ramp(path: &Path, width: u32, height: u32) {
    let mut rgba = Vec::new();
    for _ in 0..height {
        for x in 0..width {
            let value = u8::try_from(x * 255 / (width - 1)).unwrap();
            rgba.extend_from_slice(&[value, value, value, 255]);
        }
    }
    std::fs::write(path, encode_png(width, height, &rgba).unwrap()).unwrap();
}

#[test]
fn opening_a_picture_makes_its_map_at_its_size() {
    let folder = tempfile::tempdir().unwrap();
    let source = folder.path().join("floor.png");
    ramp(&source, 8, 4);
    let studio = NormalStudio::open(&source);
    assert_eq!(studio.problem, None);
    assert_eq!(studio.map().len(), 8 * 4 * 4);
    assert!(studio.map()[4 * 4] < 128, "a rising ramp faces left");
    assert_eq!(studio.map_path(), folder.path().join("floor_n.png"));
    assert!(!studio.is_saved());
}

#[test]
fn a_changed_setting_makes_the_map_again_once() {
    let folder = tempfile::tempdir().unwrap();
    let source = folder.path().join("floor.png");
    ramp(&source, 8, 4);
    let mut studio = NormalStudio::open(&source);
    let first = studio.revision();
    assert!(!studio.refresh(), "nothing moved");
    studio.settings.strength = 0.0;
    assert!(studio.refresh());
    assert_eq!(studio.revision(), first + 1);
    assert!(
        studio
            .map()
            .chunks_exact(4)
            .all(|texel| texel == FLAT_NORMAL)
    );
}

#[test]
fn saving_writes_a_map_and_settings_that_reopen_as_they_were() {
    let folder = tempfile::tempdir().unwrap();
    let source = folder.path().join("floor.png");
    ramp(&source, 8, 4);
    let mut studio = NormalStudio::open(&source);
    studio.settings = NormalMapPreset::Stone.apply(&studio.settings);
    studio.save().unwrap();
    assert!(studio.is_saved());

    let written = Picture::read(studio.map_path()).unwrap();
    assert_eq!(written.rgba, studio.map());

    for path in [source.clone(), studio.settings_path().to_path_buf()] {
        let reopened = NormalStudio::open(&path);
        assert_eq!(reopened.source(), source);
        assert_eq!(reopened.settings, studio.settings);
        assert!(reopened.is_saved());
        assert_eq!(reopened.map(), studio.map());
    }
}

#[test]
fn a_changed_picture_is_followed_with_the_same_settings() {
    let folder = tempfile::tempdir().unwrap();
    let source = folder.path().join("floor.png");
    ramp(&source, 8, 4);
    let mut studio = NormalStudio::open(&source);
    assert!(!studio.follow_disk());
    // A different picture, and a stamp that certainly differs.
    ramp(&source, 4, 4);
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
    std::fs::File::options()
        .write(true)
        .open(&source)
        .unwrap()
        .set_modified(later)
        .unwrap();
    assert!(studio.follow_disk());
    assert_eq!(studio.map().len(), 4 * 4 * 4);
}

#[test]
fn a_height_map_beside_the_picture_is_read_and_remembered() {
    let folder = tempfile::tempdir().unwrap();
    let source = folder.path().join("floor.png");
    let heights = folder.path().join("floor_height.png");
    ramp(&heights, 8, 4);
    std::fs::write(
        &source,
        encode_png(8, 4, &[90, 90, 90, 255].repeat(32)).unwrap(),
    )
    .unwrap();
    let mut studio = NormalStudio::open(&source);
    assert!(
        studio
            .map()
            .chunks_exact(4)
            .all(|texel| texel == FLAT_NORMAL)
    );
    studio.use_height_map(&heights);
    assert!(studio.map()[4 * 4] < 128);
    studio.save().unwrap();
    let reopened = NormalStudio::open(&source);
    assert_eq!(reopened.height_map_path(), Some(heights.as_path()));
    assert_eq!(reopened.map(), studio.map());
}

#[test]
fn a_missing_picture_is_a_problem_to_show_not_a_failure() {
    let folder = tempfile::tempdir().unwrap();
    let studio = NormalStudio::open(&folder.path().join("gone.png"));
    assert!(studio.problem.is_some());
    assert!(studio.map().is_empty());
}

#[test]
fn a_channel_is_shown_as_gray_with_the_maps_alpha() {
    let folder = tempfile::tempdir().unwrap();
    let source = folder.path().join("floor.png");
    ramp(&source, 8, 4);
    let studio = NormalStudio::open(&source);
    let red = studio.channel_picture(Channel::Red);
    assert_eq!(red.len(), studio.map().len());
    let value = studio.map()[16];
    assert_eq!(red[16..20], [value, value, value, 255]);
    let height = studio.channel_picture(Channel::Height);
    assert!(height[0] < height[7 * 4], "the ramp rises to the right");
}

#[test]
fn the_surface_is_the_picture_at_its_pixels_per_unit() {
    let folder = tempfile::tempdir().unwrap();
    let source = folder.path().join("floor.png");
    ramp(&source, 32, 16);
    let mut studio = NormalStudio::open(&source);
    let near = |size: [f32; 2]| (size[0] - 2.0).abs() < 1e-6 && (size[1] - 1.0).abs() < 1e-6;
    assert!(near(studio.surface_size()));
    studio.pixels_per_unit = 0.0;
    assert!(near(studio.surface_size()), "nonsense falls back to 16");
}

#[test]
fn a_file_beside_the_settings_is_named_relative_to_them() {
    assert_eq!(
        relative_to(Path::new("a/b/c/h.png"), Path::new("a/b")).as_deref(),
        Some("c/h.png")
    );
    assert_eq!(relative_to(Path::new("x/h.png"), Path::new("a")), None);
}

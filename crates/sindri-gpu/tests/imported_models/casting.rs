//! Retained hierarchy, index widths and material masks also govern model shadows.

use super::{effects::Scene, gpu, resource};
use glam::{Mat4, Quat, Vec3};
use sindri_render::{FogSettings, FrameEncodeError, ModelRenderError, RenderModel};
use std::sync::Arc;

pub(super) fn caster(
    alphas: [u8; 3],
    cutoff: Option<f32>,
    factor: f32,
    double_sided: bool,
) -> Arc<RenderModel> {
    let mut data = resource().data().clone();
    for surface in &mut data.surfaces {
        surface.texture = Some(0);
        surface.base_color = [0.8, 0.1, 0.1, factor];
        surface.alpha_cutoff = cutoff;
        surface.double_sided = double_sided;
    }
    data.images[0].width = 3;
    data.images[0].rgba = alphas
        .into_iter()
        .flat_map(|a| [255, 255, 255, a])
        .collect();
    data.images[0].mag_filter = wgpu::FilterMode::Nearest;
    data.images[0].min_filter = wgpu::FilterMode::Nearest;
    for mesh in &mut data.meshes {
        for primitive in mesh {
            for (vertex, uv) in
                primitive
                    .vertices
                    .iter_mut()
                    .zip([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]])
            {
                vertex.uv = uv;
            }
        }
    }
    Arc::new(RenderModel::new(data).unwrap())
}

pub(super) fn darkened(before: &[u8], after: &[u8]) -> usize {
    before
        .chunks_exact(4)
        .zip(after.chunks_exact(4))
        .filter(|(a, b)| a[1] > a[0].saturating_add(30) && a[1] > b[1].saturating_add(20))
        .count()
}

#[test]
fn model_casters_share_geometry_and_hierarchy_and_preserve_alpha_holes() {
    let Some(gpu) = gpu() else {
        return;
    };
    let mut counts = Vec::new();
    for (alphas, cutoff, factor) in [
        ([255; 3], Some(0.5), 1.0),
        ([255, 0, 255], Some(0.5), 1.0),
        ([0; 3], Some(0.5), 1.0),
        ([255; 3], Some(0.5), 0.0),
        ([0; 3], None, 0.0),
    ] {
        let mut scene = Scene::new(&gpu);
        scene.replace_caster(
            &caster(alphas, cutoff, factor, false),
            Mat4::from_scale_rotation_translation(
                Vec3::new(0.3, 0.3, 1.0),
                Quat::IDENTITY,
                Vec3::Z,
            ),
        );
        let clear = scene.draw(&gpu, false, 256, FogSettings::default());
        let before = scene.cubes.draw_stats();
        let shadow = scene.draw(&gpu, true, 256, FogSettings::default());
        let delta = scene.cubes.draw_stats().since(before);
        assert_eq!(
            delta.draw_calls, 8,
            "two models each have two color and two shadow primitives"
        );
        assert_eq!(delta.render_passes, 9, "including the map clear");
        assert_eq!(delta.uniform_bytes, 4 * 944 + 4 * 96);
        assert_eq!(delta.transient_geometry_uploads, 0);
        assert_eq!(
            scene.cubes.model_cache_stats().uploads,
            2,
            "color and shadow reuse the same two assets"
        );
        counts.push(darkened(&clear, &shadow));
        assert_eq!(
            scene.draw(&gpu, true, 256, FogSettings::default()),
            shadow,
            "consecutive submissions keep shadow and color uniforms independent"
        );
        assert_eq!(scene.cubes.model_cache_stats().uploads, 2);
    }
    assert!(
        counts[0] > 100,
        "opaque retained primitive instances cast shadows: {counts:?}"
    );
    assert!(
        counts[1] > 50 && counts[1] < counts[0] * 4 / 5,
        "cutout holes reduce the shadow: {counts:?}"
    );
    assert_eq!(counts[2], 0, "fully transparent mask does not cast");
    assert_eq!(
        counts[3], 0,
        "material alpha factor participates in the mask"
    );
    assert_eq!(
        counts[4], counts[0],
        "opaque mode ignores texture/material alpha"
    );
}

#[test]
fn mirrored_casters_and_double_sided_backfaces_follow_material_facing() {
    let Some(gpu) = gpu() else {
        return;
    };
    for (scale, rotation, double_sided, casts) in [
        (Vec3::new(-0.3, 0.3, 1.0), Quat::IDENTITY, false, true),
        (
            Vec3::new(0.3, 0.3, 1.0),
            Quat::from_rotation_y(std::f32::consts::PI),
            false,
            false,
        ),
        (
            Vec3::new(0.3, 0.3, 1.0),
            Quat::from_rotation_y(std::f32::consts::PI),
            true,
            true,
        ),
    ] {
        let mut scene = Scene::new(&gpu);
        scene.replace_caster(
            &caster([255; 3], None, 1.0, double_sided),
            Mat4::from_scale_rotation_translation(scale, rotation, Vec3::Z),
        );
        let clear = scene.draw(&gpu, false, 256, FogSettings::default());
        let shadow = scene.draw(&gpu, true, 256, FogSettings::default());
        let count = darkened(&clear, &shadow);
        if casts {
            assert!(
                count > 100,
                "mirrored/double-sided caster stays visible: {count}"
            );
        } else {
            assert_eq!(count, 0, "single-sided backfaces do not cast");
        }
    }
}

/// A caster scaled to nothing draws its frame and casts nothing: a game
/// shrinks a figure away on purpose, and refusing the frame stopped it.
#[test]
fn a_caster_scaled_to_nothing_casts_nothing_and_stops_nothing() {
    let Some(gpu) = gpu() else {
        return;
    };
    let mut scene = Scene::new(&gpu);
    scene.replace_caster(
        &caster([255; 3], None, 1.0, false),
        Mat4::from_scale(Vec3::ZERO),
    );
    let clear = scene.draw(&gpu, false, 256, FogSettings::default());
    let shadow = scene
        .draw_result(&gpu, true, 256, FogSettings::default())
        .expect("a model scaled to nothing is skipped, not an error");
    assert_eq!(darkened(&clear, &shadow), 0, "nothing is there to cast");
}

#[test]
fn a_caster_whose_transform_is_not_finite_returns_the_frame_error() {
    let Some(gpu) = gpu() else {
        return;
    };
    let mut scene = Scene::new(&gpu);
    scene.replace_caster(
        &caster([255; 3], None, 1.0, false),
        Mat4::from_translation(Vec3::splat(f32::NAN)),
    );
    assert!(matches!(
        scene.draw_result(&gpu, true, 256, FogSettings::default()),
        Err(FrameEncodeError::Model(ModelRenderError::InvalidTransform))
    ));
}

#[test]
fn authored_material_alpha_updates_model_color_and_shadow_without_uploads() {
    let Some(gpu) = gpu() else {
        return;
    };
    let mut scene = Scene::new(&gpu);
    scene.replace_caster(
        &caster([255; 3], Some(0.5), 1.0, false),
        Mat4::from_scale_rotation_translation(Vec3::new(0.3, 0.3, 1.0), Quat::IDENTITY, Vec3::Z),
    );
    for (alpha, casts) in [(0.0, false), (1.0, true), (0.0, false)] {
        scene.set_caster_material(sindri_render::MaterialFactors {
            base_color: [1.0, 1.0, 1.0, alpha],
            ..Default::default()
        });
        let clear = scene.draw(&gpu, false, 256, FogSettings::default());
        let shadow = scene.draw(&gpu, true, 256, FogSettings::default());
        let count = darkened(&clear, &shadow);
        let visible = clear[(64 * 128 + 72) * 4];
        if casts {
            assert!(visible > 150, "opaque override draws the caster");
        } else {
            assert!(visible < 80, "masked override exposes the background");
        }
        if casts {
            assert!(count > 100);
        } else {
            assert_eq!(count, 0);
        }
    }
    assert_eq!(scene.cubes.model_cache_stats().uploads, 2);
}

#[test]
fn imported_emission_does_not_change_masked_shadow_coverage() {
    let Some(gpu) = gpu() else {
        return;
    };
    let base = caster([255, 0, 255], Some(0.5), 1.0, false);
    let mut data = base.data().clone();
    for surface in &mut data.surfaces {
        surface.emissive = [0.0, 0.0, 1.0];
    }
    let emitting = Arc::new(RenderModel::new(data).unwrap());
    let model =
        Mat4::from_scale_rotation_translation(Vec3::new(0.3, 0.3, 1.0), Quat::IDENTITY, Vec3::Z);
    let mut images = Vec::new();
    for asset in [&base, &emitting] {
        let mut scene = Scene::new(&gpu);
        scene.replace_caster(asset, model);
        let clear = scene.draw(&gpu, false, 256, FogSettings::default());
        let shadow = scene.draw(&gpu, true, 256, FogSettings::default());
        assert!(darkened(&clear, &shadow) > 50);
        images.push(shadow);
    }
    let mut checked = 0;
    for (before, after) in images[0].chunks_exact(4).zip(images[1].chunks_exact(4)) {
        if before[1] > before[0].saturating_add(30) {
            assert_eq!(
                before, after,
                "emission leaves the green receiver's shadow unchanged"
            );
            checked += 1;
        }
    }
    assert!(checked > 100);
    assert_ne!(images[0], images[1], "the caster itself emits blue");
}

//! The silhouette changes hidden pixels while visible fragments keep their art.
use super::{effects::Scene, gpu};
use glam::{Mat4, Vec3};
use sindri_render::{FogSettings, FrameCommand};

#[test]
fn silhouette_reveals_only_fragments_behind_opaque_geometry() {
    let Some(gpu) = gpu() else {
        return;
    };
    let mut scene = Scene::new(&gpu);
    scene.edit_caster(|pass| {
        if let FrameCommand::TexturedCube { model, .. } = &mut pass.command {
            *model = Mat4::from_translation(Vec3::new(-0.5, 0.0, 1.0))
                * Mat4::from_scale(Vec3::splat(0.5));
        }
    });
    let before = scene.draw(&gpu, false, 256, FogSettings::default());
    scene.show_occluded_subject();
    let after = scene.draw(&gpu, false, 256, FogSettings::default());
    let changed = before
        .chunks_exact(4)
        .zip(after.chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count();
    let blue_added = before
        .chunks_exact(4)
        .zip(after.chunks_exact(4))
        .filter(|(a, b)| b[2] > a[2].saturating_add(30))
        .count();
    assert!(changed > 50, "hidden silhouette drawn: {changed}");
    assert_eq!(changed, blue_added, "only the blue overlay changes pixels");
    assert_eq!(scene.cubes.model_cache_stats().uploads, 1);
}

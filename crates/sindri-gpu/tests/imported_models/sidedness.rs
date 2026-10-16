//! The same authored facing policy governs shadows for every surface path.
use super::{
    casting::{caster, darkened},
    effects::Scene,
    gpu,
};
use glam::{Mat4, Quat, Vec3};
use sindri_render::{
    CachedMeshId, CachedTexturedMeshUpload, FogSettings, FrameCommand, MaterialSidedness,
    MeshSurface, RenderMesh, TexturedVertex,
};
use std::sync::Arc;

#[test]
fn sidedness_controls_reflected_mesh_and_model_shadows_without_reuploads() {
    let Some(gpu) = gpu() else { return };
    for x in [0.9, -0.9] {
        for kind in 0..4 {
            let mut scene = Scene::new(&gpu);
            let asset = caster([255; 3], None, 1.0, true);
            let model = Mat4::from_scale_rotation_translation(
                Vec3::new(x, 0.9, 1.0),
                Quat::from_rotation_y(std::f32::consts::PI),
                Vec3::Z,
            );
            let vertices: Vec<_> = asset.data().meshes[0][0]
                .vertices
                .iter()
                .map(|v| TexturedVertex::new(v.position, v.uv).with_normal(v.normal))
                .collect();
            scene.edit_caster(|pass| {
                let FrameCommand::TexturedCube { texture, .. } = pass.command else {
                    panic!()
                };
                pass.command = match kind {
                    0 => FrameCommand::TexturedMesh {
                        model,
                        texture,
                        vertices: vertices.clone(),
                        indices: vec![0, 1, 2, 0, 2, 3],
                    },
                    1 => FrameCommand::RetainedTexturedMesh {
                        model,
                        texture,
                        geometry: Arc::new(
                            RenderMesh::new(vertices.clone(), vec![0, 1, 2, 0, 2, 3]).unwrap(),
                        ),
                    },
                    2 => FrameCommand::CachedTexturedMesh {
                        model,
                        texture,
                        cache: CachedMeshId::new(10),
                        revision: 1,
                        replacement: Some(CachedTexturedMeshUpload::new(
                            vertices.clone(),
                            vec![0, 1, 2, 0, 2, 3],
                        )),
                        surface: MeshSurface::default(),
                    },
                    _ => FrameCommand::Model {
                        model,
                        asset: Arc::clone(&asset),
                    },
                };
            });
            for value in [
                MaterialSidedness::Inherit,
                MaterialSidedness::Double,
                MaterialSidedness::Single,
                MaterialSidedness::Inherit,
            ] {
                scene.edit_caster(|pass| pass.sidedness = value);
                let clear = scene.draw(&gpu, false, 256, FogSettings::default());
                let shadow = scene.draw(&gpu, true, 256, FogSettings::default());
                let count = darkened(&clear, &shadow);
                let casts = value == MaterialSidedness::Double
                    || (value == MaterialSidedness::Inherit && kind == 3);
                if casts {
                    assert!(
                        count > 50,
                        "kind {kind}, scale {x}, policy {value:?}: {count} shadow pixels"
                    );
                } else {
                    assert_eq!(count, 0, "single-sided back faces do not cast");
                }
            }
            assert_eq!(
                scene.cubes.model_cache_stats().uploads,
                if kind == 3 { 2 } else { 1 }
            );
            assert_eq!(
                scene.cubes.cached_mesh_stats().buffer_uploads,
                u64::from(kind == 1 || kind == 2)
            );
        }
    }
}

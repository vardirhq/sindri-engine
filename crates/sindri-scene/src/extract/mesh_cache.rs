//! Geometry revisions are independent of transform/material/camera revisions.
use super::SceneExtractError;
use crate::{MeshComponent, TextureBindings, components::SurfaceMesh};
use sindri_core::{EntityId, SpriteRef, World};
use sindri_render::{RenderMesh, TextureId, TexturedVertex, UvRect};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Debug)]
struct Surface {
    revision: u64,
    textures: u64,
    source: SurfaceMesh,
    rect: UvRect,
    texture: TextureId,
    geometry: Arc<RenderMesh>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct SurfaceCache(BTreeMap<EntityId, Surface>);

impl SurfaceCache {
    pub(super) fn retain(&mut self, live: &BTreeSet<EntityId>) {
        self.0.retain(|id, _| live.contains(id));
    }

    pub(super) fn resolve(
        &mut self,
        world: &World,
        entity: EntityId,
        mesh: &MeshComponent,
        textures: &TextureBindings,
    ) -> Result<Option<(TextureId, Arc<RenderMesh>)>, SceneExtractError> {
        let Some(source) = &mesh.surface else {
            return Ok(None);
        };
        if source.vertices.len() != source.uvs.len() {
            return Ok(None);
        }
        let revision = world.revision(entity).unwrap_or_default();
        if let Some(entry) = self.0.get(&entity)
            && entry.revision == revision
            && entry.textures == textures.generation()
        {
            return Ok(Some((entry.texture, Arc::clone(&entry.geometry))));
        }
        let reference = SpriteRef::parse(&mesh.texture)?;
        let (texture, rect) = textures.resolve_sprite(&reference);
        if let Some(entry) = self.0.get_mut(&entity)
            && entry.source == *source
            && entry.rect == rect
        {
            entry.revision = revision;
            entry.textures = textures.generation();
            entry.texture = texture;
            return Ok(Some((texture, Arc::clone(&entry.geometry))));
        }
        let vertices = source
            .vertices
            .iter()
            .zip(&source.uvs)
            .enumerate()
            .map(|(index, (position, uv))| {
                TexturedVertex::new(
                    *position,
                    [
                        rect.width().mul_add(uv[0], rect.x()),
                        rect.height().mul_add(uv[1], rect.y()),
                    ],
                )
                .with_normal(
                    source
                        .normals
                        .as_ref()
                        .and_then(|n| n.get(index))
                        .copied()
                        .unwrap_or([0.0; 3]),
                )
            })
            .collect();
        let geometry = Arc::new(RenderMesh::new(vertices, source.indices.clone())?);
        self.0.insert(
            entity,
            Surface {
                revision,
                textures: textures.generation(),
                source: source.clone(),
                rect,
                texture,
                geometry: Arc::clone(&geometry),
            },
        );
        Ok(Some((texture, geometry)))
    }
}

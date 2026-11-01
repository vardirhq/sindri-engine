//! CPU-side picking for renderable entities in an authoring viewport.
//!
//! Picking uses the exact view-projection matrix used to draw the frame and
//! the same local geometry as the renderers. It is an authoring concern, used
//! by the desktop editor's Scene view and Sindri Mobile's canvas, and stays a
//! pure CPU query: selecting an entity must not add IDs or readback buffers to
//! a game frame. It lives here, beside voxel block picking, because both
//! authoring hosts need it and neither may depend on the other.

use crate::{
    MeshComponent, MeshPrimitive, OverlayPlacement, OverlayView, SpriteComponent, TilemapComponent,
    UiHierarchy, UiImageComponent,
};
use glam::{Mat4, Quat, Vec3};
use sindri_core::{ComponentRegistryError, ComponentSchemaRegistry, EntityId, Transform3D, World};

#[derive(Clone, Copy, Debug)]
struct Hit {
    entity: EntityId,
    depth: f32,
    layer: i32,
}

/// Selects the topmost world-space renderable under a normalized viewport point.
///
/// Opaque geometry blocks sprites behind it. Among visible transparent
/// renderables, the higher authored layer wins; distance and then entity index
/// break ties in the same direction as the renderer's back-to-front ordering.
pub fn pick_world(
    world: &World,
    components: &ComponentSchemaRegistry,
    view_projection: Mat4,
    point: [f32; 2],
) -> Result<Option<EntityId>, ComponentRegistryError> {
    let Some(ray) = RaySegment::through_viewport(view_projection, point) else {
        return Ok(None);
    };

    let nearest_mesh = components
        .query::<MeshComponent>(world)?
        .into_iter()
        .filter_map(|(entity, mesh)| {
            let transform = transform_of(world, entity);
            mesh_hit(ray, transform, mesh.primitive).map(|depth| Hit {
                entity,
                depth,
                layer: mesh.layer,
            })
        })
        .min_by(nearer_hit);

    let mesh_depth = nearest_mesh.map_or(f32::INFINITY, |hit| hit.depth);
    let sprites = components
        .query::<SpriteComponent>(world)?
        .into_iter()
        .filter(|(_, sprite)| is_drawn(sprite.tint))
        .filter_map(|(entity, sprite)| {
            let transform = transform_of(world, entity);
            plane_hit(ray, transform, 0.5).map(|(depth, _)| Hit {
                entity,
                depth,
                layer: sprite.layer,
            })
        });
    let tilemaps = components
        .query::<TilemapComponent>(world)?
        .into_iter()
        .filter_map(|(entity, map)| {
            let transform = transform_of(world, entity);
            let (depth, local) = plane_hit(ray, transform, f32::INFINITY)?;
            let (column, row) = map.local_to_tile(local.x, local.y)?;
            map.tile(column, row)?;
            Some(Hit {
                entity,
                depth,
                layer: map.layer,
            })
        });

    let transparent = sprites
        .chain(tilemaps)
        .filter(|hit| hit.depth < mesh_depth)
        .max_by(frontmost_transparent);
    Ok(transparent.or(nearest_mesh).map(|hit| hit.entity))
}

/// Where an entity covers the viewport: left, top, right and bottom, each a
/// fraction across it, in the units [`pick_world`] takes a point in.
///
/// The same local geometry a pick hits — the sprite quad, the unit cube, the
/// tilemap's cells — so an outline drawn from it encloses exactly what a tap
/// on it would select. `None` for an entity with none of them, one switched
/// off, or one that does not reach the viewport.
pub fn world_bounds(
    world: &World,
    components: &ComponentSchemaRegistry,
    view_projection: Mat4,
    entity: EntityId,
) -> Result<Option<[f32; 4]>, ComponentRegistryError> {
    let corners: Vec<Vec3> = if let Some(map) = components.get::<TilemapComponent>(world, entity)? {
        if map.grid_bounds().is_err() {
            return Ok(None);
        }
        let [width, height] = map.tile_size;
        let last = [map.columns.saturating_sub(1), map.rows.saturating_sub(1)];
        [[0, 0], [last[0], 0], [0, last[1]], last]
            .into_iter()
            .flat_map(|[column, row]| {
                let [x, y] = map.tile_to_local(column, row);
                [[-0.5, -0.5], [0.5, -0.5], [-0.5, 0.5], [0.5, 0.5]]
                    .map(|[dx, dy]| Vec3::new(x + dx * width, y + dy * height, 0.0))
            })
            .collect()
    } else if components
        .get::<SpriteComponent>(world, entity)?
        .is_some_and(|sprite| is_drawn(sprite.tint))
    {
        [[-0.5, -0.5], [0.5, -0.5], [-0.5, 0.5], [0.5, 0.5]]
            .map(|[x, y]| Vec3::new(x, y, 0.0))
            .to_vec()
    } else if components
        .get::<MeshComponent>(world, entity)?
        .is_some_and(|mesh| mesh.primitive == MeshPrimitive::Cube)
    {
        (0..8)
            .map(|corner| {
                let at = |bit: u32| if corner & bit == 0 { -0.5 } else { 0.5 };
                Vec3::new(at(1), at(2), at(4))
            })
            .collect()
    } else {
        return Ok(None);
    };
    let model = view_projection * transform_matrix(transform_of(world, entity));
    let mut bounds = [
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    for corner in corners {
        let clip = model * corner.extend(1.0);
        if clip.w <= f32::EPSILON {
            return Ok(None);
        }
        let x = (clip.x / clip.w + 1.0) * 0.5;
        let y = (1.0 - clip.y / clip.w) * 0.5;
        bounds = [
            bounds[0].min(x),
            bounds[1].min(y),
            bounds[2].max(x),
            bounds[3].max(y),
        ];
    }
    let reaches = bounds.iter().all(|edge| edge.is_finite())
        && bounds[2] >= 0.0
        && bounds[0] <= 1.0
        && bounds[3] >= 0.0
        && bounds[1] <= 1.0;
    Ok(reaches.then_some(bounds))
}

/// Selects the topmost UI element under a normalized viewport point.
///
/// A pass of its own, because UI is not in the world. It is laid out against
/// the viewport — an anchor picks a point, and the transform is an offset from
/// it — so a world ray through a world camera passes nowhere near it. Twelve of
/// Gather's twenty-two entities are UI, and none of them could be clicked in
/// the view that draws them.
///
/// Images are hit-tested from their transform and anchor, the way the overlay
/// places them. Strings are not: what a string covers is decided by glyph
/// layout inside the text renderer, so their boxes are measured there and
/// handed in as [`TextTarget`]s rather than guessed here — a guessed box picks
/// the wrong thing near its edges, which is worse than not picking at all.
pub fn pick_ui(
    world: &World,
    components: &ComponentSchemaRegistry,
    overlay: OverlayView,
    placement: &OverlayPlacement,
    point: [f32; 2],
    texts: &[TextTarget],
) -> Result<Option<EntityId>, ComponentRegistryError> {
    let Some(ray) = RaySegment::through_viewport(overlay.view_projection, point) else {
        return Ok(None);
    };
    // The same placement the frame drew from: an image parented to a panel is
    // where the panel put it, not where its own offset from the screen would be.
    let hierarchy = UiHierarchy::of(world, components)?;
    let images = components
        .query::<UiImageComponent>(world)?
        .into_iter()
        .filter(|(_, image)| is_drawn(image.tint))
        .filter_map(|(entity, image)| {
            let placed = hierarchy.placement_or(entity, image.anchor);
            let model = placement.place(placed, transform_of(world, entity));
            plane_hit_model(ray, model, 0.5).map(|(depth, _)| Hit {
                entity,
                depth,
                layer: image.layer,
            })
        });
    let strings = texts
        .iter()
        .filter(|text| text.covers(point))
        .map(|text| Hit {
            entity: text.entity,
            // Flat, like every other overlay hit, so the comparison below decides.
            depth: 0.0,
            layer: text.layer,
        });
    Ok(images
        .chain(strings)
        // The overlay is flat, so depth separates nothing: what decides which
        // of two overlapping elements is on top is the layer, then the entity
        // index, in the same direction the renderer stacks them.
        .max_by(frontmost_transparent)
        .map(|hit| hit.entity))
}

/// One laid-out string as a box in the viewport.
///
/// Measured by whatever holds the text renderer and handed to [`pick_ui`],
/// because the renderer owns the font system and this module owns no GPU.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextTarget {
    pub entity: EntityId,
    pub layer: i32,
    /// Left, top, right and bottom, each a fraction across the viewport — the
    /// same units a pick point is in.
    pub bounds: [f32; 4],
}

impl TextTarget {
    fn covers(&self, point: [f32; 2]) -> bool {
        let [left, top, right, bottom] = self.bounds;
        (left..=right).contains(&point[0]) && (top..=bottom).contains(&point[1])
    }
}

#[derive(Clone, Copy, Debug)]
struct RaySegment {
    near: Vec3,
    far: Vec3,
}

impl RaySegment {
    fn through_viewport(view_projection: Mat4, point: [f32; 2]) -> Option<Self> {
        if !(0.0..=1.0).contains(&point[0]) || !(0.0..=1.0).contains(&point[1]) {
            return None;
        }
        let inverse = view_projection.inverse();
        if !matrix_is_finite(inverse) {
            return None;
        }
        let x = point[0] * 2.0 - 1.0;
        let y = 1.0 - point[1] * 2.0;
        let near = inverse.project_point3(Vec3::new(x, y, 0.0));
        let far = inverse.project_point3(Vec3::new(x, y, 1.0));
        (near.is_finite() && far.is_finite()).then_some(Self { near, far })
    }

    fn in_local_space(self, transform: Transform3D) -> Option<Self> {
        self.in_model_space(transform_matrix(transform))
    }

    /// The same, for something placed by a matrix rather than by a transform.
    ///
    /// A UI element is one: where it lands is its transform *and* its anchor
    /// *and* the viewport's shape, which only the overlay can put together.
    fn in_model_space(self, model: Mat4) -> Option<Self> {
        let inverse = model.inverse();
        if !matrix_is_finite(inverse) {
            return None;
        }
        Some(Self {
            near: inverse.transform_point3(self.near),
            far: inverse.transform_point3(self.far),
        })
    }
}

fn mesh_hit(ray: RaySegment, transform: Transform3D, primitive: MeshPrimitive) -> Option<f32> {
    match primitive {
        MeshPrimitive::Cube => cube_hit(ray.in_local_space(transform)?),
        _ => None,
    }
}

/// Intersects the renderer's unit cube, whose local vertices span `[-0.5, 0.5]`.
fn cube_hit(ray: RaySegment) -> Option<f32> {
    let direction = ray.far - ray.near;
    let mut entering = 0.0_f32;
    let mut leaving = 1.0_f32;
    for (origin, direction) in [
        (ray.near.x, direction.x),
        (ray.near.y, direction.y),
        (ray.near.z, direction.z),
    ] {
        if direction.abs() <= f32::EPSILON {
            if !(-0.5..=0.5).contains(&origin) {
                return None;
            }
            continue;
        }
        let first = (-0.5 - origin) / direction;
        let second = (0.5 - origin) / direction;
        entering = entering.max(first.min(second));
        leaving = leaving.min(first.max(second));
        if entering > leaving {
            return None;
        }
    }
    (entering <= 1.0 && leaving >= 0.0).then_some(entering.max(0.0))
}

/// Intersects a local Z=0 plane, optionally bounded like the sprite quad.
fn plane_hit(ray: RaySegment, transform: Transform3D, half_extent: f32) -> Option<(f32, Vec3)> {
    plane_hit_model(ray, transform_matrix(transform), half_extent)
}

/// The same, against a model matrix that was not built from a transform alone.
fn plane_hit_model(ray: RaySegment, model: Mat4, half_extent: f32) -> Option<(f32, Vec3)> {
    let ray = ray.in_model_space(model)?;
    let direction = ray.far - ray.near;
    if direction.z.abs() <= f32::EPSILON {
        return None;
    }
    let depth = -ray.near.z / direction.z;
    if !(0.0..=1.0).contains(&depth) {
        return None;
    }
    let point = ray.near + direction * depth;
    (point.x.abs() <= half_extent && point.y.abs() <= half_extent).then_some((depth, point))
}

/// Whether a tint puts anything on the screen at all.
///
/// A fully transparent element is drawn as nothing, and clicking nothing must
/// not select it. Gather's win banner is exactly this: `tint` alpha zero, a
/// third of the viewport wide, sitting in the middle of the scene until the
/// game says otherwise. Picked, it would swallow every click in the centre of
/// the view and select an element nobody can see.
///
/// Exactly zero rather than a threshold: anything above it is visible, however
/// faintly, and a faint thing is still a thing someone aimed at. What is hidden
/// this way is still selectable from the hierarchy, which is where an invisible
/// entity is reachable at all.
///
/// Public because a string's color is the same question, and it is asked
/// where strings are measured rather than here.
pub fn is_drawn(tint: [f32; 4]) -> bool {
    tint[3] > 0.0
}

fn transform_of(world: &World, entity: EntityId) -> Transform3D {
    world.world_transform(entity).unwrap_or_default()
}

fn transform_matrix(transform: Transform3D) -> Mat4 {
    Mat4::from_scale_rotation_translation(
        Vec3::from_array(transform.scale),
        Quat::from_array(transform.rotation),
        Vec3::from_array(transform.position),
    )
}

fn matrix_is_finite(matrix: Mat4) -> bool {
    matrix.to_cols_array().into_iter().all(f32::is_finite)
}

fn nearer_hit(left: &Hit, right: &Hit) -> std::cmp::Ordering {
    left.depth
        .total_cmp(&right.depth)
        .then_with(|| left.layer.cmp(&right.layer))
        .then_with(|| left.entity.index().cmp(&right.entity.index()))
}

fn frontmost_transparent(left: &Hit, right: &Hit) -> std::cmp::Ordering {
    left.layer
        .cmp(&right.layer)
        .then_with(|| right.depth.total_cmp(&left.depth))
        .then_with(|| left.entity.index().cmp(&right.entity.index()))
}

#[cfg(test)]
mod tests;

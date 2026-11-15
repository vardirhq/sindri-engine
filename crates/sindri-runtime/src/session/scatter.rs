//! The things a land's scatter stands near play (`sindri.scatter`), made from
//! their prefabs as play comes near and unmade once it has gone on.
//!
//! Where each one stands is the seed's (`sindri_scene::scatter_placements`);
//! this keeps the ones within reach made. They are ordinary entities: what
//! stops a walker, lights and casts a shadow does so as any placed by hand
//! would. One that disappears while it is made was taken (a script despawned
//! it), and the scatter remembers it in `taken`, so it does not grow back.
//!
//! A thing is unmade a few squares past its reach rather than at it, so
//! walking back and forth across the edge does not make and unmake the same
//! tree every step.

use std::collections::{BTreeMap, HashMap};

use glam::{Mat4, Vec3, Vec4};
use sindri_core::{ComponentSchemaRegistry, EntityId, SceneComponent, SceneEntityId, World};
use sindri_scene::{ScatterComponent, VoxelGround, VoxelWorldComponent};

use super::Session;

/// How far past its reach a thing is kept before it is unmade, in squares.
const KEEP_PAST_REACH: i32 = 4;

/// What one land's scatter has made.
#[derive(Clone)]
struct Dressed {
    /// The world the ground was read from, so an edit reads it again.
    world: VoxelWorldComponent,
    ground: VoxelGround,
    /// The square play was in when the things were last worked out.
    middle: Option<[i32; 2]>,
    /// The scatter they were worked out from.
    scatter: ScatterComponent,
    /// Each thing made, by its key, and its root.
    made: HashMap<[i32; 3], EntityId>,
}

/// Every land's scatter.
#[derive(Clone, Default)]
pub(super) struct Scattered(BTreeMap<EntityId, Dressed>);

impl Session {
    /// Makes what is near play on every scattered land and unmakes what is
    /// far, and remembers what a script took.
    pub(super) fn step_scatter(
        &mut self,
        world: &mut World,
        viewport: (f32, f32),
        problems: &mut Vec<String>,
    ) {
        let lands: Vec<(EntityId, ScatterComponent, VoxelWorldComponent)> =
            match self.components.query::<ScatterComponent>(world) {
                Ok(found) => found
                    .into_iter()
                    .filter(|(entity, _)| world.is_active(*entity))
                    .filter_map(|(entity, scatter)| {
                        let land = self.components.get::<VoxelWorldComponent>(world, entity);
                        Some((entity, scatter, land.ok()??))
                    })
                    .collect(),
                Err(error) => {
                    problems.push(format!("Scatter: {error}"));
                    return;
                }
            };
        self.scattered
            .0
            .retain(|entity, _| lands.iter().any(|(kept, _, _)| kept == entity));
        for (entity, scatter, land) in lands {
            if let Err(error) = scatter.validate() {
                problems.push(format!("Scatter on {entity:?}: {error}"));
                continue;
            }
            let Some(middle) = middle_of_play(world, &self.components, entity, &scatter, viewport)
            else {
                continue;
            };
            if let Err(problem) = self.dress(world, entity, scatter, land, middle) {
                problems.push(format!("Scatter on {entity:?}: {problem}"));
            }
        }
    }

    fn dress(
        &mut self,
        world: &mut World,
        entity: EntityId,
        mut scatter: ScatterComponent,
        land: VoxelWorldComponent,
        middle: [i32; 2],
    ) -> Result<(), String> {
        let tile_sets = (!self.tile_sets.is_empty()).then_some(&self.tile_sets);
        let stale = self
            .scattered
            .0
            .get(&entity)
            .is_none_or(|dressed| dressed.world != land);
        if stale {
            let ground = VoxelGround::of(&land, tile_sets).map_err(|error| error.to_string())?;
            let made = self
                .scattered
                .0
                .remove(&entity)
                .map(|dressed| dressed.made)
                .unwrap_or_default();
            self.scattered.0.insert(
                entity,
                Dressed {
                    world: land,
                    ground,
                    middle: None,
                    scatter: scatter.clone(),
                    made,
                },
            );
        }
        let dressed = self.scattered.0.get_mut(&entity).expect("inserted above");

        // What a script took since the last step is remembered as taken.
        let taken: Vec<[i32; 3]> = dressed
            .made
            .iter()
            .filter(|(_, root)| !world.contains(**root))
            .map(|(key, _)| *key)
            .collect();
        if !taken.is_empty() {
            for key in &taken {
                dressed.made.remove(key);
            }
            scatter.taken.extend(taken);
            let payload = serde_json::to_value(&scatter).map_err(|error| error.to_string())?;
            if let Some(data) = world.get_mut(entity) {
                data.components
                    .insert(ScatterComponent::TYPE_NAME.to_owned(), payload);
            }
        }

        if dressed.middle == Some(middle) && dressed.scatter == scatter {
            return Ok(());
        }
        dressed.middle = Some(middle);
        dressed.scatter.clone_from(&scatter);

        let reach = i32::try_from(scatter.reach).unwrap_or(i32::MAX / 2);
        let near = sindri_scene::scatter_placements(
            &scatter,
            &dressed.ground,
            [middle[0] - reach, middle[1] - reach],
            [middle[0] + reach, middle[1] + reach],
        );
        let keep = reach + KEEP_PAST_REACH;
        let far: Vec<[i32; 3]> = dressed
            .made
            .keys()
            .filter(|key| {
                (key[0] - middle[0]).abs() > keep
                    || (key[1] - middle[1]).abs() > keep
                    || !near.iter().any(|placement| placement.key == **key)
                        && (key[0] - middle[0]).abs() <= reach
                        && (key[1] - middle[1]).abs() <= reach
            })
            .copied()
            .collect();
        for key in far {
            if let Some(root) = dressed.made.remove(&key) {
                world
                    .despawn_recursive(root)
                    .map_err(|error| error.to_string())?;
            }
        }
        let root = land_matrix(world, entity);
        let mut missing = Vec::new();
        for placement in near {
            if dressed.made.contains_key(&placement.key) {
                continue;
            }
            let Some(prefab) = self.prefabs.get(&placement.prefab) else {
                missing.push(placement.prefab);
                continue;
            };
            let made = stand(world, prefab, &self.prefabs, root, &placement)?;
            dressed.made.insert(placement.key, made);
        }
        missing.sort();
        missing.dedup();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(format!("no prefab {}", missing.join(", ")))
        }
    }
}

/// Makes one thing from its prefab and stands it on its square, mirrored if
/// it is; its root.
fn stand(
    world: &mut World,
    prefab: &sindri_core::PrefabDocument,
    prefabs: &sindri_decay::PrefabSources,
    land: Mat4,
    placement: &sindri_scene::Placement,
) -> Result<EntityId, String> {
    let made = world
        .spawn_prefab_from(prefab, prefabs)
        .map_err(|error| format!("{}: {error}", placement.prefab))?;
    #[allow(clippy::cast_precision_loss)]
    let at = land.transform_point3(Vec3::new(
        placement.key[0] as f32 + 0.5 + placement.offset[0],
        -(placement.key[1] as f32) - 0.5 + placement.offset[1],
        0.0,
    ));
    if let Some(data) = world.get_mut(made.root) {
        // The prefab's own placing is kept, as an offset from the square.
        let mut transform = data.transform_3d.unwrap_or_default();
        transform.position = [
            at.x + transform.position[0],
            at.y + transform.position[1],
            at.z + transform.position[2],
        ];
        if placement.flipped {
            transform.scale[0] = -transform.scale[0];
        }
        data.transform_3d = Some(transform);
    }
    Ok(made.root)
}

/// The land's own placing in the world, which its squares are laid out in.
fn land_matrix(world: &World, entity: EntityId) -> Mat4 {
    world.world_transform(entity).map_or(Mat4::IDENTITY, |t| {
        Mat4::from_scale_rotation_translation(
            Vec3::from_array(t.scale),
            glam::Quat::from_array(t.rotation),
            Vec3::from_array(t.position),
        )
    })
}

/// The square play is in: under the entity the scatter is `around`, or else
/// where the middle of the world camera's view meets the land.
fn middle_of_play(
    world: &World,
    components: &ComponentSchemaRegistry,
    land: EntityId,
    scatter: &ScatterComponent,
    viewport: (f32, f32),
) -> Option<[i32; 2]> {
    let to_land = land_matrix(world, land).inverse();
    let local = if scatter.around.trim().is_empty() {
        let aspect = if viewport.1 > 0.0 {
            viewport.0 / viewport.1
        } else {
            1.0
        };
        let camera = sindri_scene::world_camera_of(world, components, aspect).ok()??;
        let to_world = camera.view_projection.inverse();
        let near = to_land * to_world * Vec4::new(0.0, 0.0, 0.0, 1.0);
        let far = to_land * to_world * Vec4::new(0.0, 0.0, 1.0, 1.0);
        let (near, far) = (near.truncate() / near.w, far.truncate() / far.w);
        let along = far.z - near.z;
        if along.abs() < f32::EPSILON {
            return None;
        }
        near + (far - near) * (-near.z / along)
    } else {
        let id = SceneEntityId::new(scatter.around.trim()).ok()?;
        let around = world.entity_for_source_id(&id)?;
        let at = world.world_transform(around)?.position;
        to_land.transform_point3(Vec3::from_array(at))
    };
    if !local.x.is_finite() || !local.y.is_finite() {
        return None;
    }
    #[allow(clippy::cast_possible_truncation)]
    Some([local.x.floor() as i32, (-local.y).floor() as i32])
}

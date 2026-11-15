//! Walls where a map's ground cannot be walked on (`sindri.map_walls`).
//!
//! A voxel world drawn as a map is only a picture to 2D physics: a walker
//! crosses its rivers and its sea as if they were grass. This puts that right
//! where it matters, near play. The runtime raises a static 2D collider over
//! every run of squares in a row that a walker cannot stand on (water, lava,
//! nothing at all) within `reach` of play, sixteen squares by sixteen, and
//! takes them down once play has gone on, so a land as wide as the generator
//! is walled where someone is and nowhere else.

use serde::{Deserialize, Serialize};
use sindri_core::SceneComponent;

/// The walls on a voxel world's unwalkable ground, near play.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MapWallsComponent {
    /// The entity whose place is the middle of what is walled, by its scene
    /// ID. Empty, the world camera's view.
    pub around: String,
    /// How many squares out from the middle walls stand.
    pub reach: u32,
    /// The collision layers the walls are on, and what they stop: the same
    /// bits a 2D collider's `layers` takes.
    pub memberships: u32,
    pub filter: u32,
}

impl Default for MapWallsComponent {
    fn default() -> Self {
        Self {
            around: String::new(),
            reach: 24,
            memberships: 1,
            filter: u32::MAX,
        }
    }
}

impl SceneComponent for MapWallsComponent {
    const TYPE_NAME: &'static str = "sindri.map_walls";
}

/// The furthest out walls reach, in squares.
pub const MOST_WALL_REACH: u32 = 128;

impl MapWallsComponent {
    /// The walls, or why they would wall nothing sensible.
    ///
    /// # Errors
    /// A reach of nothing, or past [`MOST_WALL_REACH`].
    pub fn validate(&self) -> Result<(), String> {
        if self.reach == 0 || self.reach > MOST_WALL_REACH {
            return Err(format!("`reach` must be from 1 to {MOST_WALL_REACH}"));
        }
        Ok(())
    }
}

impl MapWallsComponent {
    /// What one wall is made of: a static body and a box `half_extents` across,
    /// on these walls' layers, as a scene authors them.
    #[must_use]
    pub fn wall_components(&self, half_extents: [f32; 2]) -> Vec<(String, serde_json::Value)> {
        let body = sindri_physics::RigidBody2d {
            kind: sindri_physics::RigidBodyKind::Static,
            ..sindri_physics::RigidBody2d::default()
        };
        let mut collider = sindri_physics::Collider2d::rectangle(half_extents);
        collider.layers = sindri_physics::CollisionLayers::new(self.memberships, self.filter);
        collider.friction = 0.0;
        [
            ("sindri.physics2d.rigid_body", serde_json::to_value(body)),
            ("sindri.physics2d.collider", serde_json::to_value(collider)),
        ]
        .into_iter()
        .filter_map(|(name, value)| Some((name.to_owned(), value.ok()?)))
        .collect()
    }
}

/// The runs of squares in one row of a map that a walker cannot stand on,
/// from `first` to `last` column inclusive: each as its first and last column.
///
/// `walkable` answers for a square; `None` is a square with nothing to stand
/// on at all, which is walled like water.
pub fn unwalkable_runs(
    first: i32,
    last: i32,
    mut walkable: impl FnMut(i32) -> Option<bool>,
) -> Vec<[i32; 2]> {
    let mut runs = Vec::new();
    let mut start = None;
    for column in first..=last {
        let blocked = walkable(column) != Some(true);
        match (blocked, start) {
            (true, None) => start = Some(column),
            (false, Some(from)) => {
                runs.push([from, column - 1]);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(from) = start {
        runs.push([from, last]);
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_is_walled_in_runs_of_what_cannot_be_walked() {
        let ground = [true, false, false, true, true, false, true];
        let runs = unwalkable_runs(0, 6, |column| {
            ground.get(usize::try_from(column).ok()?).copied()
        });
        assert_eq!(runs, vec![[1, 2], [5, 5]]);
        let open_ends = unwalkable_runs(0, 3, |column| (column == 1).then_some(true));
        assert_eq!(
            open_ends,
            vec![[0, 0], [2, 3]],
            "nothing to stand on is a wall too"
        );
        assert!(unwalkable_runs(0, 3, |_| Some(true)).is_empty());
    }

    /// A wall is a component pair the scene reads back as a static box.
    #[test]
    fn a_wall_is_a_static_box_on_its_layers() {
        let walls = MapWallsComponent {
            memberships: 1,
            filter: 6,
            ..MapWallsComponent::default()
        };
        let parts: std::collections::BTreeMap<_, _> =
            walls.wall_components([2.0, 0.5]).into_iter().collect();
        let collider: crate::Collider2dComponent =
            serde_json::from_value(parts["sindri.physics2d.collider"].clone()).expect("a collider");
        assert_eq!(collider.0[0].layers.filter, 6);
        let body: crate::RigidBody2dComponent =
            serde_json::from_value(parts["sindri.physics2d.rigid_body"].clone()).expect("a body");
        assert_eq!(body.0.kind, sindri_physics::RigidBodyKind::Static);
    }

    #[test]
    fn walls_that_reach_nothing_are_refused() {
        assert!(MapWallsComponent::default().validate().is_ok());
        let none = MapWallsComponent {
            reach: 0,
            ..MapWallsComponent::default()
        };
        assert!(none.validate().is_err());
    }
}

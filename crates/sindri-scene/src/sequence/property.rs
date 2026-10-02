//! Which number a track moves, and reaching it on an entity.

use sindri_core::{EntityData, EntityId, World};

use super::SequenceError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    const fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
}

/// A number a track can move.
///
/// Written as text in the sequence:
///
/// | Text | Moves |
/// | --- | --- |
/// | `position.x`, `position.y`, `position.z` | the transform's position |
/// | `rotation` | the turn about the view axis, in degrees |
/// | `scale` | the transform's scale on x and y together |
/// | `scale.x`, `scale.y`, `scale.z` | one axis of the scale |
/// | `sindri.sprite/tint.3` | a component, then a path into it: field names and list positions |
///
/// `rotation` sets a turn about z alone, which is what a 2D scene has; a
/// tilted 3D object moved by it loses its tilt.
#[derive(Clone, Debug, PartialEq)]
pub enum Property {
    Position(Axis),
    Rotation,
    Scale,
    ScaleAxis(Axis),
    Field { component: String, path: Vec<Step> },
}

/// One step into a component: a field by name or a list by position.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    Field(String),
    Index(usize),
}

/// Every transform channel, in the order an editor offers them.
pub const TRANSFORM_PROPERTIES: [&str; 8] = [
    "position.x",
    "position.y",
    "position.z",
    "rotation",
    "scale",
    "scale.x",
    "scale.y",
    "scale.z",
];

impl Property {
    pub fn parse(text: &str) -> Result<Self, SequenceError> {
        let refuse = || SequenceError::Property(text.to_owned());
        let axis = |name: &str| match name {
            "x" => Some(Axis::X),
            "y" => Some(Axis::Y),
            "z" => Some(Axis::Z),
            _ => None,
        };
        if let Some((component, path)) = text.split_once('/') {
            if component.is_empty() || path.is_empty() {
                return Err(refuse());
            }
            let path = path
                .split('.')
                .map(|step| {
                    if step.is_empty() {
                        return Err(refuse());
                    }
                    Ok(step
                        .parse::<usize>()
                        .map_or_else(|_| Step::Field(step.to_owned()), Step::Index))
                })
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(Self::Field {
                component: component.to_owned(),
                path,
            });
        }
        match text.split_once('.') {
            None if text == "rotation" => Ok(Self::Rotation),
            None if text == "scale" => Ok(Self::Scale),
            Some(("position", name)) => axis(name).map(Self::Position).ok_or_else(refuse),
            Some(("scale", name)) => axis(name).map(Self::ScaleAxis).ok_or_else(refuse),
            _ => Err(refuse()),
        }
    }

    /// The number as the entity holds it now.
    pub fn read(&self, entity: &EntityData) -> Option<f32> {
        let transform = entity.transform_3d.unwrap_or_default();
        Some(match self {
            Self::Position(axis) => transform.position[axis.index()],
            Self::Rotation => z_degrees(transform.rotation),
            Self::Scale => transform.scale[0],
            Self::ScaleAxis(axis) => transform.scale[axis.index()],
            #[allow(clippy::cast_possible_truncation)]
            Self::Field { component, path } => {
                walk(entity.components.get(component)?, path)?.as_f64()? as f32
            }
        })
    }

    /// Sets the number on the entity.
    pub fn write(&self, entity: &mut EntityData, value: f32) -> Result<(), SequenceError> {
        let mut transform = entity.transform_3d.unwrap_or_default();
        match self {
            Self::Position(axis) => transform.position[axis.index()] = value,
            Self::Rotation => transform.rotation = about_z(value),
            Self::Scale => {
                transform.scale[0] = value;
                transform.scale[1] = value;
            }
            Self::ScaleAxis(axis) => transform.scale[axis.index()] = value,
            Self::Field { component, path } => {
                let missing = || SequenceError::Field(format!("{component}/{}", dotted(path)));
                let mut at = entity.components.get_mut(component).ok_or_else(missing)?;
                for step in path {
                    at = match step {
                        Step::Field(name) => at.get_mut(name.as_str()),
                        Step::Index(index) => at.get_mut(*index),
                    }
                    .ok_or_else(missing)?;
                }
                if !at.is_number() {
                    return Err(missing());
                }
                *at = serde_json::Value::from(f64::from(value));
                return Ok(());
            }
        }
        entity.transform_3d = Some(transform);
        Ok(())
    }
}

fn walk<'a>(value: &'a serde_json::Value, path: &[Step]) -> Option<&'a serde_json::Value> {
    path.iter().try_fold(value, |at, step| match step {
        Step::Field(name) => at.get(name.as_str()),
        Step::Index(index) => at.get(*index),
    })
}

fn dotted(path: &[Step]) -> String {
    path.iter()
        .map(|step| match step {
            Step::Field(name) => name.clone(),
            Step::Index(index) => index.to_string(),
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// A turn of `degrees` about z, as `[x, y, z, w]`.
fn about_z(degrees: f32) -> [f32; 4] {
    let (sin, cos) = (degrees.to_radians() * 0.5).sin_cos();
    [0.0, 0.0, sin, cos]
}

/// The turn about z a quaternion makes, in degrees.
fn z_degrees(rotation: [f32; 4]) -> f32 {
    let [x, y, z, w] = rotation;
    let siny = 2.0 * w.mul_add(z, x * y);
    let cosy = 2.0f32.mul_add(-y.mul_add(y, z * z), 1.0);
    siny.atan2(cosy).to_degrees()
}

/// The entity a target path names, below `from`.
pub fn resolve(world: &World, from: EntityId, target: &str) -> Result<EntityId, SequenceError> {
    let mut at = from;
    for name in target.split('/').filter(|name| !name.is_empty()) {
        let data = world
            .get(at)
            .ok_or_else(|| SequenceError::Target(target.to_owned()))?;
        at = data
            .children
            .iter()
            .copied()
            .find(|child| {
                world
                    .get(*child)
                    .is_some_and(|child| child.name.as_deref() == Some(name))
            })
            .ok_or_else(|| SequenceError::Target(target.to_owned()))?;
    }
    Ok(at)
}

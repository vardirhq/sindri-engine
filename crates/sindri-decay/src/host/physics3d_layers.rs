//! Named query masks from the active authored 3D world, independent of 2D.

use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};
use sindri_core::SceneComponent;
use sindri_scene::PhysicsWorld3dComponent;

use super::WorldHost;
use crate::surface::physics3d::Physics3dCall;

impl WorldHost<'_> {
    pub(super) fn physics3d_layers(
        &self,
        call: Physics3dCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        if args.len() != 1 {
            return Err(error("incorrect argument count"));
        }
        if self.physics3d.is_none() {
            return Err(error("host has no 3D physics"));
        }
        let names: Vec<&Value> = match (call, &args[0]) {
            (Physics3dCall::Layer, Value::String(_)) => vec![&args[0]],
            (Physics3dCall::Mask, Value::Array(names)) => names.iter().collect(),
            _ => {
                return Err(error(
                    "requires a layer name or list of layer names as text",
                ));
            }
        };
        let mut settings = self
            .world
            .entities()
            .filter(|(entity, _)| self.world.is_active(*entity))
            .filter_map(|(_, data)| data.components.get(PhysicsWorld3dComponent::TYPE_NAME));
        let payload = settings.next();
        if settings.next().is_some() {
            return Err(error("multiple active 3D physics world settings"));
        }
        let layers = payload
            .map(|payload| serde_json::from_value::<PhysicsWorld3dComponent>(payload.clone()))
            .transpose()
            .map_err(|failure| error(&format!("invalid 3D world settings: {failure}")))?
            .map_or_else(Vec::new, |settings| settings.layers);
        let mut mask = 0_u32;
        for name in names {
            let Value::String(name) = name else {
                return Err(error("layer names must be text"));
            };
            let bit = sindri_scene::layer_bit(&layers, name)
                .ok_or_else(|| error(&format!("no 3D collision layer is called '{name}'")))?;
            mask |= bit;
        }
        Ok(Value::Number(f64::from(mask)))
    }
}

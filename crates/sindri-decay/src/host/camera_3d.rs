//! Projection and orbit controls select an explicit camera entity.

use super::{
    WorldHost,
    convert::{as_f32, number},
    physics3d::vector,
};
use crate::surface::CameraCall;
use decay_ir::Path;
use decay_runtime::{RuntimeError, Value};

impl WorldHost<'_> {
    pub(super) fn camera_3d_call(
        &mut self,
        call: CameraCall,
        path: &Path,
        args: &[Value],
    ) -> Result<Value, RuntimeError> {
        let error = |message: &str| RuntimeError::Host(format!("{}: {message}", path.dotted()));
        let camera = self.entity_argument(path, args, 0, "the camera")?;
        let numeric = |value: &Value| {
            let value = number(path, value)?;
            if !value.is_finite() || value.abs() > f64::from(f32::MAX) {
                return Err(error("numbers must be finite and within f32 range"));
            }
            Ok(as_f32(value))
        };
        let changed = match (call, args) {
            (CameraCall::PerspectiveFov, [_, degrees]) => {
                sindri_scene::set_camera_perspective_fov(self.world, camera, numeric(degrees)?)
            }
            (CameraCall::Orbit, [_, _, yaw, pitch, distance]) => {
                let target = self.entity_argument(path, args, 1, "the orbit target")?;
                sindri_scene::set_camera_orbit(
                    self.world,
                    camera,
                    target,
                    numeric(yaw)?,
                    numeric(pitch)?,
                    numeric(distance)?,
                )
            }
            (CameraCall::OrbitOffset, [_, offset]) => {
                sindri_scene::set_camera_orbit_offset(self.world, camera, vector(path, offset)?)
            }
            (CameraCall::OrbitSmoothing, [_, smoothing]) => {
                sindri_scene::set_camera_orbit_smoothing(self.world, camera, numeric(smoothing)?)
            }
            (CameraCall::OrbitCollision, [_, mask, padding]) => {
                let mask = super::raycast::filter(
                    path,
                    &[mask.clone(), Value::Bool(false), Value::Null],
                    0,
                )?
                .mask;
                sindri_scene::set_camera_orbit_collision(
                    self.world,
                    camera,
                    mask,
                    numeric(padding)?,
                )
            }
            (CameraCall::ClearOrbit, [_]) => sindri_scene::clear_camera_orbit(self.world, camera),
            _ => return Err(error("incorrect camera arguments")),
        };
        if !changed {
            return Err(error(
                "requires a valid camera, orbit target and settings; FOV requires a perspective camera and degrees strictly between 0 and 180",
            ));
        }
        Ok(Value::Unit)
    }
}

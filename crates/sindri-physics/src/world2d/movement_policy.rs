//! Per-piece one-way filtering shared by controller phases.

use rapier2d::parry::shape::Shape;

use super::one_way::SUPPORT_SLOP;
use super::r2;
use crate::{OneWay2d, RaycastFilter2d};

#[derive(Clone, Copy)]
pub(super) struct MovementProbe2d<'a> {
    pub shape: &'a dyn Shape,
    pub pose: r2::Pose,
    pub filter: RaycastFilter2d,
    /// None is ordinary geometry; Some(false) respects policy; Some(true) drops.
    pub one_way: Option<bool>,
}

impl MovementProbe2d<'_> {
    pub(super) fn allows(
        self,
        piece: &dyn Shape,
        pose: r2::Pose,
        policy: Option<OneWay2d>,
        direction: r2::Vector,
        contact_normal: r2::Vector,
    ) -> bool {
        let (Some(drop), Some(policy)) = (self.one_way, policy) else {
            return true;
        };
        if drop {
            return false;
        }
        let normal = pose.rotation.transform_vector(
            r2::Vector::from_array(policy.normal) / policy.normal[0].hypot(policy.normal[1]),
        );
        if direction.dot(normal) > f32::EPSILON
            || contact_normal.dot(normal) + f32::EPSILON < policy.angle.cos()
        {
            return false;
        }
        let (Some(front), Some(back)) = (piece.as_support_map(), self.shape.as_support_map())
        else {
            return false;
        };
        let top = front.support_point(&pose, normal);
        let bottom = back.support_point(&self.pose, -normal);
        (bottom - top).dot(normal) >= -SUPPORT_SLOP
    }
}

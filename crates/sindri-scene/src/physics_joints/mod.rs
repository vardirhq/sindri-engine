//! Authored constraints resolve after every body has synchronized.
mod components;
mod reference;
mod sync;

pub use components::{
    DistanceJoint2dComponent, HingeJoint2dComponent, SliderJoint2dComponent, SpringJoint2dComponent,
};
pub(crate) use sync::SceneJoints2d;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod prefab_tests;

#[cfg(test)]
mod placed_tests;

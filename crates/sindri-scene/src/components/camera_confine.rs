//! Viewport-aware bounds for an unrotated 2D orthographic camera.

use sindri_core::{EntityId, SceneComponent, World};

use super::{CameraBounds, CameraComponent, CameraFit};

pub(super) fn effective_bounds(
    world: &World,
    entity: EntityId,
    bounds: CameraBounds,
    viewport: Option<(f32, f32)>,
) -> CameraBounds {
    if !bounds.contain_view {
        return bounds;
    }
    let Some(half) = view_half_size(world, entity, viewport) else {
        return bounds;
    };
    let mut effective = bounds;
    for (axis, inset) in half.into_iter().enumerate() {
        let min = bounds.min[axis] + inset;
        let max = bounds.max[axis] - inset;
        if min <= max {
            effective.min[axis] = min;
            effective.max[axis] = max;
        } else {
            let center = (bounds.min[axis] + bounds.max[axis]) * 0.5;
            effective.min[axis] = center;
            effective.max[axis] = center;
        }
    }
    effective
}

pub(super) fn view_half_size(
    world: &World,
    entity: EntityId,
    viewport: Option<(f32, f32)>,
) -> Option<[f32; 2]> {
    let (width, height) = viewport?;
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return None;
    }
    let camera = world
        .get(entity)?
        .components
        .get(CameraComponent::TYPE_NAME)?;
    let CameraComponent::Orthographic {
        vertical_size, fit, ..
    } = serde_json::from_value(camera.clone()).ok()?
    else {
        return None;
    };
    if !vertical_size.is_finite() || vertical_size <= 0.0 {
        return None;
    }
    let aspect = width / height;
    if !aspect.is_finite() || aspect <= 0.0 {
        return None;
    }
    let half_height = match fit {
        CameraFit::Shorter if aspect < 1.0 => vertical_size * 0.5 / aspect,
        CameraFit::Shorter | CameraFit::Height => vertical_size * 0.5,
    };
    Some([half_height * aspect, half_height])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sindri_core::EntityData;

    #[test]
    fn bounds_fit_portrait_landscape_and_oversized_views() {
        let mut world = World::default();
        let mut camera = EntityData::default();
        camera.components.insert(
            CameraComponent::TYPE_NAME.into(),
            json!({"projection":"orthographic", "vertical_size":11.25,
                "near":0.1, "far":100.0, "fit":"height"}),
        );
        let camera = world.spawn(camera);
        let bounds = CameraBounds {
            min: [0.0, 0.0],
            max: [64.0, 14.0],
            contain_view: true,
        };
        let landscape = effective_bounds(&world, camera, bounds, Some((960.0, 540.0)));
        assert_eq!(landscape.min, [10.0, 5.625]);
        assert_eq!(landscape.max, [54.0, 8.375]);
        let portrait = effective_bounds(&world, camera, bounds, Some((390.0, 844.0)));
        assert!((portrait.min[0] - 2.599).abs() < 0.01);
        assert!((portrait.min[1] - 5.625).abs() < f32::EPSILON);
        let ultrawide = effective_bounds(&world, camera, bounds, Some((2520.0, 1080.0)));
        assert!((ultrawide.min[0] - 13.125).abs() < 0.001);
        assert!((ultrawide.max[0] - 50.875).abs() < 0.001);
        let wide = effective_bounds(&world, camera, bounds, Some((7000.0, 500.0)));
        assert_eq!((wide.min[0], wide.max[0]), (32.0, 32.0));
    }

    #[test]
    fn runtime_bounds_edits_preserve_the_containment_mode() {
        let mut world = World::default();
        let mut camera = EntityData::default();
        camera.components.insert(
            CameraComponent::TYPE_NAME.into(),
            json!({"projection":"orthographic", "vertical_size":11.25,
                "near":0.1, "far":100.0, "fit":"height"}),
        );
        camera.components.insert(
            super::super::CameraBehaviorComponent::TYPE_NAME.into(),
            json!({"confine":{"min":[0.0,0.0], "max":[64.0,14.0], "contain_view":true}}),
        );
        let camera = world.spawn(camera);
        assert!(crate::set_camera_bounds(
            &mut world,
            [1.0, 2.0],
            [50.0, 13.0]
        ));
        let behavior = &world.get(camera).unwrap().components
            [super::super::CameraBehaviorComponent::TYPE_NAME];
        assert_eq!(behavior["confine"]["contain_view"], json!(true));
        assert_eq!(behavior["confine"]["min"], json!([1.0, 2.0]));
    }
}

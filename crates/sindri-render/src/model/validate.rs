use glam::Mat4;

use super::{ModelData, ModelIndexData, ModelNodeInstance, ModelRenderError};

fn invalid(message: &str) -> ModelRenderError {
    ModelRenderError::InvalidData(message.into())
}

pub(super) fn transform(matrix: Mat4) -> Result<(), ModelRenderError> {
    if !matrix.is_finite() || matrix.determinant().abs() < 1e-12 {
        return Err(ModelRenderError::InvalidTransform);
    }
    Ok(())
}

pub(super) fn data(data: &ModelData) -> Result<(), ModelRenderError> {
    for mesh in &data.meshes {
        for primitive in mesh {
            if primitive.vertices.is_empty()
                || primitive.material.is_some_and(|i| i >= data.surfaces.len())
                || !primitive.vertices.iter().all(|v| {
                    v.position
                        .iter()
                        .chain(&v.normal)
                        .chain(&v.uv)
                        .all(|x| x.is_finite())
                })
            {
                return Err(invalid("invalid primitive vertices or material reference"));
            }
            let valid = match &primitive.indices {
                ModelIndexData::U16(values) => {
                    !values.is_empty()
                        && values.len().is_multiple_of(3)
                        && values
                            .iter()
                            .all(|v| usize::from(*v) < primitive.vertices.len())
                }
                ModelIndexData::U32(values) => {
                    !values.is_empty()
                        && values.len().is_multiple_of(3)
                        && values.iter().all(|v| {
                            usize::try_from(*v).is_ok_and(|i| i < primitive.vertices.len())
                        })
                }
            };
            if !valid {
                return Err(invalid(
                    "indices must reference vertices and form triangles",
                ));
            }
        }
    }
    for surface in &data.surfaces {
        if surface.texture.is_some_and(|i| i >= data.images.len())
            || !surface
                .base_color
                .into_iter()
                .chain([surface.metallic, surface.roughness])
                .chain(surface.alpha_cutoff)
                .all(|v| v.is_finite() && (0.0..=1.0).contains(&v))
        {
            return Err(invalid("invalid material values or texture reference"));
        }
    }
    for image in &data.images {
        if image.width == 0
            || image.height == 0
            || image
                .width
                .checked_mul(image.height)
                .and_then(|n| n.checked_mul(4))
                .and_then(|n| usize::try_from(n).ok())
                != Some(image.rgba.len())
        {
            return Err(invalid("invalid RGBA image dimensions"));
        }
    }
    Ok(())
}

pub(super) fn hierarchy(data: &ModelData) -> Result<Vec<ModelNodeInstance>, ModelRenderError> {
    let mut parents = vec![0_u8; data.nodes.len()];
    for node in &data.nodes {
        transform(node.local_transform)?;
        if node.mesh.is_some_and(|i| i >= data.meshes.len()) {
            return Err(invalid("node references missing mesh"));
        }
        for &child in &node.children {
            let Some(count) = parents.get_mut(child) else {
                return Err(invalid("node references missing child"));
            };
            *count = count.saturating_add(1);
            if *count != 1 {
                return Err(invalid("node has multiple parents"));
            }
        }
    }
    // Validate all nodes, including nodes outside the selected glTF scene.
    let mut visited = vec![false; data.nodes.len()];
    let mut stack: Vec<_> = parents
        .iter()
        .enumerate()
        .filter(|(_, count)| **count == 0)
        .map(|(i, _)| i)
        .collect();
    while let Some(index) = stack.pop() {
        visited[index] = true;
        stack.extend(&data.nodes[index].children);
    }
    if visited.contains(&false) {
        return Err(invalid("node hierarchy contains a cycle"));
    }
    let mut seen = vec![false; data.nodes.len()];
    if data
        .roots
        .iter()
        .any(|&i| parents.get(i).is_none_or(|count| *count != 0))
    {
        return Err(invalid("scene references a missing root or a child"));
    }
    let mut stack: Vec<_> = data
        .roots
        .iter()
        .rev()
        .map(|&i| (i, Mat4::IDENTITY))
        .collect();
    let mut instances = Vec::new();
    while let Some((index, parent)) = stack.pop() {
        let Some(node) = data.nodes.get(index) else {
            return Err(invalid("scene references missing root"));
        };
        if seen[index] {
            return Err(invalid("scene roots overlap or reference a child"));
        }
        seen[index] = true;
        let matrix = parent * node.local_transform;
        transform(matrix)?;
        if let Some(mesh) = node.mesh {
            instances.push(ModelNodeInstance {
                node: index,
                mesh,
                transform: matrix,
            });
        }
        stack.extend(node.children.iter().rev().map(|&child| (child, matrix)));
    }
    Ok(instances)
}

use super::{
    ModelIndices, ModelPrimitive,
    parse::{invalid, unsupported},
};
use crate::{AssetBytes, AssetDecodeError};
use gltf::{
    Semantic,
    mesh::{Mode, util::ReadIndices},
};

pub(super) fn primitive(
    bytes: &AssetBytes,
    blob: &[u8],
    primitive: &gltf::Primitive<'_>,
) -> Result<ModelPrimitive, AssetDecodeError> {
    if primitive.mode() != Mode::Triangles {
        return Err(unsupported(bytes, "only triangle primitives are supported"));
    }
    if primitive.morph_targets().next().is_some() {
        return Err(unsupported(bytes, "morph targets are not supported"));
    }
    validate_attributes(bytes, primitive)?;
    let reader = primitive.reader(|_| Some(blob));
    let positions: Vec<_> = reader
        .read_positions()
        .ok_or_else(|| invalid(bytes, "primitive has no readable POSITION accessor"))?
        .collect();
    let indices = match reader.read_indices() {
        Some(ReadIndices::U8(values)) => ModelIndices::U16(values.map(u16::from).collect()),
        Some(ReadIndices::U16(values)) => ModelIndices::U16(values.collect()),
        Some(ReadIndices::U32(values)) => ModelIndices::U32(values.collect()),
        None if primitive.indices().is_some() => {
            return Err(invalid(bytes, "primitive has an unreadable index accessor"));
        }
        None => ModelIndices::U32(
            (0..positions.len())
                .map(u32::try_from)
                .collect::<Result<_, _>>()
                .map_err(|_| invalid(bytes, "primitive has too many vertices"))?,
        ),
    };
    let flat_indices = indices.to_u32();
    if positions.is_empty()
        || flat_indices.is_empty()
        || !flat_indices.len().is_multiple_of(3)
        || flat_indices
            .iter()
            .any(|&i| usize::try_from(i).map_or(true, |i| i >= positions.len()))
    {
        return Err(invalid(
            bytes,
            "triangle indices are empty, incomplete, or out of range",
        ));
    }
    let normals: Vec<[f32; 3]> = match reader.read_normals() {
        Some(normals) => normals.collect(),
        None if primitive.get(&Semantic::Normals).is_some() => {
            return Err(invalid(
                bytes,
                "primitive has an unreadable NORMAL accessor",
            ));
        }
        None => generated_normals(&positions, &flat_indices),
    };
    let uvs: Vec<[f32; 2]> = match reader.read_tex_coords(0) {
        Some(uvs) => uvs.into_f32().collect(),
        None if primitive.get(&Semantic::TexCoords(0)).is_some() => {
            return Err(invalid(
                bytes,
                "primitive has an unreadable TEXCOORD_0 accessor",
            ));
        }
        None if primitive
            .material()
            .pbr_metallic_roughness()
            .base_color_texture()
            .is_some() =>
        {
            return Err(invalid(
                bytes,
                "textured primitive has no TEXCOORD_0 accessor",
            ));
        }
        None => vec![[0.0; 2]; positions.len()],
    };
    let value = ModelPrimitive {
        positions,
        normals,
        uvs,
        indices,
        material: primitive.material().index(),
    };
    if value.normals.len() != value.positions.len()
        || value.uvs.len() != value.positions.len()
        || !value
            .positions
            .iter()
            .flatten()
            .chain(value.normals.iter().flatten())
            .chain(value.uvs.iter().flatten())
            .all(|v| v.is_finite())
    {
        return Err(invalid(
            bytes,
            "vertex attributes must have matching lengths and finite values",
        ));
    }
    Ok(value)
}

fn generated_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0; 3]; positions.len()];
    for triangle in indices.chunks_exact(3) {
        let ids = [triangle[0], triangle[1], triangle[2]]
            .map(|i| usize::try_from(i).expect("indices validated against vertex count"));
        let [origin, point_b, point_c] = ids.map(|i| positions[i]);
        let edge_b = [
            point_b[0] - origin[0],
            point_b[1] - origin[1],
            point_b[2] - origin[2],
        ];
        let edge_c = [
            point_c[0] - origin[0],
            point_c[1] - origin[1],
            point_c[2] - origin[2],
        ];
        let normal = [
            edge_b[1] * edge_c[2] - edge_b[2] * edge_c[1],
            edge_b[2] * edge_c[0] - edge_b[0] * edge_c[2],
            edge_b[0] * edge_c[1] - edge_b[1] * edge_c[0],
        ];
        for i in ids {
            for (component, add) in normals[i].iter_mut().zip(normal) {
                *component += add;
            }
        }
    }
    for normal in &mut normals {
        let length = normal.iter().map(|v| v * v).sum::<f32>().sqrt();
        if length > f32::EPSILON {
            for component in normal {
                *component /= length;
            }
        }
    }
    normals
}

fn validate_attributes(
    bytes: &AssetBytes,
    primitive: &gltf::Primitive<'_>,
) -> Result<(), AssetDecodeError> {
    use gltf::accessor::{DataType, Dimensions};
    for (semantic, accessor) in primitive.attributes() {
        if !matches!(
            semantic,
            Semantic::Positions | Semantic::Normals | Semantic::TexCoords(0)
        ) {
            return Err(unsupported(
                bytes,
                format!("vertex attribute {semantic:?} is not supported"),
            ));
        }
        let valid = match semantic {
            Semantic::Positions | Semantic::Normals => {
                accessor.data_type() == DataType::F32
                    && accessor.dimensions() == Dimensions::Vec3
                    && !accessor.normalized()
            }
            Semantic::TexCoords(0) => {
                accessor.dimensions() == Dimensions::Vec2
                    && (accessor.data_type() == DataType::F32
                        || (matches!(accessor.data_type(), DataType::U8 | DataType::U16)
                            && accessor.normalized()))
            }
            _ => false,
        };
        if !valid {
            return Err(invalid(
                bytes,
                format!("invalid accessor format for {semantic:?}"),
            ));
        }
    }
    if primitive.indices().is_some_and(|a| {
        a.dimensions() != Dimensions::Scalar
            || !matches!(a.data_type(), DataType::U8 | DataType::U16 | DataType::U32)
            || a.normalized()
    }) {
        return Err(invalid(
            bytes,
            "indices require an unsigned scalar accessor",
        ));
    }
    Ok(())
}

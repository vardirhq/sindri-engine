use gltf::{Gltf, material::AlphaMode};
use sindri_core::AssetLoadErrorKind;

use super::super::AssetDecodeError;
use super::{ModelAsset, ModelMaterial, ModelMesh, ModelNode, ModelTexture};
use crate::{AssetBytes, AssetDecoder, TextureAssetDecoder};

pub(super) fn error(
    bytes: &AssetBytes,
    kind: AssetLoadErrorKind,
    message: impl Into<String>,
) -> AssetDecodeError {
    AssetDecodeError::new(bytes.id().clone(), "model", kind, message)
}

pub(super) fn unsupported(bytes: &AssetBytes, message: impl Into<String>) -> AssetDecodeError {
    error(bytes, AssetLoadErrorKind::UnsupportedFormat, message)
}

pub(super) fn invalid(bytes: &AssetBytes, message: impl Into<String>) -> AssetDecodeError {
    error(bytes, AssetLoadErrorKind::InvalidData, message)
}

pub(super) fn decode(bytes: &AssetBytes) -> Result<ModelAsset, AssetDecodeError> {
    if !bytes.as_slice().starts_with(b"glTF") {
        return Err(unsupported(bytes, "expected a binary GLB 2.0 asset"));
    }
    let gltf = Gltf::from_slice(bytes.as_slice()).map_err(|cause| {
        let kind = if matches!(&cause, gltf::Error::Validation(errors) if errors.iter().all(|(_, error)| *error == gltf::json::validation::Error::Unsupported)) {
            AssetLoadErrorKind::UnsupportedFormat
        } else { AssetLoadErrorKind::InvalidData };
        error(bytes, kind, cause.to_string())
    })?;
    let blob = gltf
        .blob
        .as_deref()
        .ok_or_else(|| invalid(bytes, "GLB has no BIN chunk"))?;
    validate_buffers(bytes, &gltf, blob)?;
    if gltf.skins().next().is_some() {
        return Err(unsupported(bytes, "skinning is not supported"));
    }
    if let Some(extension) = gltf.extensions_required().next() {
        return Err(unsupported(
            bytes,
            format!("required extension '{extension}' is not supported"),
        ));
    }
    let mut warnings: Vec<_> = gltf
        .extensions_used()
        .map(|name| format!("optional extension '{name}' is ignored"))
        .collect();
    if gltf.animations().next().is_some() {
        warnings.push("animations are ignored; the authored static pose is imported".into());
    }
    if gltf.cameras().next().is_some() {
        warnings.push("glTF cameras are ignored; use a Sindri scene camera".into());
    }
    let materials = gltf
        .materials()
        .map(|m| material(bytes, &m))
        .collect::<Result<_, _>>()?;
    let textures = gltf
        .textures()
        .map(|t| texture(bytes, blob, &t))
        .collect::<Result<_, _>>()?;
    let meshes = gltf
        .meshes()
        .map(|mesh| {
            Ok(ModelMesh {
                name: mesh.name().map(str::to_owned),
                primitives: mesh
                    .primitives()
                    .map(|p| super::geometry::primitive(bytes, blob, &p))
                    .collect::<Result<_, _>>()?,
            })
        })
        .collect::<Result<_, AssetDecodeError>>()?;
    let nodes: Vec<_> = gltf
        .nodes()
        .map(|node| ModelNode {
            name: node.name().map(str::to_owned),
            local_transform: node.transform().matrix(),
            children: node.children().map(|child| child.index()).collect(),
            mesh: node.mesh().map(|mesh| mesh.index()),
        })
        .collect();
    let scene = gltf
        .default_scene()
        .or_else(|| gltf.scenes().next())
        .ok_or_else(|| invalid(bytes, "model has no scene"))?;
    let roots = scene.nodes().map(|node| node.index()).collect();
    validate_nodes(bytes, &nodes)?;
    Ok(ModelAsset {
        nodes,
        roots,
        meshes,
        materials,
        textures,
        warnings,
    })
}

fn validate_buffers(bytes: &AssetBytes, gltf: &Gltf, blob: &[u8]) -> Result<(), AssetDecodeError> {
    for buffer in gltf.buffers() {
        if buffer.index() != 0 || !matches!(buffer.source(), gltf::buffer::Source::Bin) {
            return Err(unsupported(
                bytes,
                "external buffers are not supported; embed them in the GLB",
            ));
        }
        if buffer.length() > blob.len() {
            return Err(invalid(bytes, "buffer exceeds the GLB BIN chunk"));
        }
    }
    for view in gltf.views() {
        if view
            .offset()
            .checked_add(view.length())
            .is_none_or(|end| end > blob.len())
        {
            return Err(invalid(
                bytes,
                format!("buffer view {} exceeds the BIN chunk", view.index()),
            ));
        }
    }
    for accessor in gltf.accessors() {
        if accessor.sparse().is_some() {
            return Err(unsupported(
                bytes,
                format!("sparse accessor {} is not supported", accessor.index()),
            ));
        }
        let view = accessor
            .view()
            .ok_or_else(|| invalid(bytes, "accessor has no buffer view"))?;
        let stride = view.stride().unwrap_or(accessor.size());
        let required = accessor
            .count()
            .checked_sub(1)
            .and_then(|count| count.checked_mul(stride))
            .and_then(|size| size.checked_add(accessor.size()))
            .unwrap_or(usize::MAX);
        if stride < accessor.size()
            || accessor
                .offset()
                .checked_add(required)
                .is_none_or(|end| end > view.length())
        {
            return Err(invalid(
                bytes,
                format!("accessor {} exceeds its buffer view", accessor.index()),
            ));
        }
    }
    Ok(())
}

fn material(
    bytes: &AssetBytes,
    material: &gltf::Material<'_>,
) -> Result<ModelMaterial, AssetDecodeError> {
    if material.alpha_mode() == AlphaMode::Blend {
        return Err(unsupported(
            bytes,
            format!(
                "material {:?}: alpha blending is not supported",
                material.name()
            ),
        ));
    }
    let pbr = material.pbr_metallic_roughness();
    if pbr.base_color_texture().is_some_and(|t| t.tex_coord() != 0) {
        return Err(unsupported(bytes, "base-color textures require TEXCOORD_0"));
    }
    if material.normal_texture().is_some()
        || material.occlusion_texture().is_some()
        || material.emissive_texture().is_some()
        || pbr.metallic_roughness_texture().is_some()
    {
        return Err(unsupported(
            bytes,
            "normal, occlusion, emissive and metallic/roughness textures are not supported",
        ));
    }
    let value = ModelMaterial {
        name: material.name().map(str::to_owned),
        base_color: pbr.base_color_factor(),
        base_color_texture: pbr.base_color_texture().map(|t| t.texture().index()),
        metallic: pbr.metallic_factor(),
        roughness: pbr.roughness_factor(),
        double_sided: material.double_sided(),
        alpha_cutoff: (material.alpha_mode() == AlphaMode::Mask)
            .then(|| material.alpha_cutoff().unwrap_or(0.5)),
    };
    if !value
        .base_color
        .iter()
        .chain([&value.metallic, &value.roughness])
        .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
    {
        return Err(invalid(
            bytes,
            "material factors must be finite and within 0..1",
        ));
    }
    Ok(value)
}

fn texture(
    bytes: &AssetBytes,
    blob: &[u8],
    texture: &gltf::Texture<'_>,
) -> Result<ModelTexture, AssetDecodeError> {
    let gltf::image::Source::View { view, mime_type } = texture.source().source() else {
        return Err(unsupported(
            bytes,
            "external images are not supported; embed PNG/JPEG images in the GLB",
        ));
    };
    if !matches!(mime_type, "image/png" | "image/jpeg") {
        return Err(unsupported(
            bytes,
            format!("image MIME type '{mime_type}' is not supported"),
        ));
    }
    let image = TextureAssetDecoder.decode(AssetBytes::new(
        bytes.id().clone(),
        blob[view.offset()..view.offset() + view.length()].to_vec(),
    ))?;
    let sampler = texture.sampler();
    Ok(ModelTexture {
        image,
        wrap_s: sampler.wrap_s().as_gl_enum(),
        wrap_t: sampler.wrap_t().as_gl_enum(),
        mag_filter: sampler.mag_filter().map(|f| f.as_gl_enum()),
        min_filter: sampler.min_filter().map(|f| f.as_gl_enum()),
    })
}

fn validate_nodes(bytes: &AssetBytes, nodes: &[ModelNode]) -> Result<(), AssetDecodeError> {
    let mut parents = vec![None; nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        if !node.local_transform.iter().flatten().all(|v| v.is_finite()) {
            return Err(invalid(
                bytes,
                format!("node {index} has a non-finite transform"),
            ));
        }
        for &child in &node.children {
            if child >= nodes.len() || parents[child].replace(index).is_some() {
                return Err(invalid(
                    bytes,
                    "node has multiple parents or an invalid child",
                ));
            }
        }
    }
    for index in 0..nodes.len() {
        let mut current = parents[index];
        let mut steps = 0;
        while let Some(parent) = current {
            steps += 1;
            if steps > nodes.len() {
                return Err(invalid(bytes, "node hierarchy contains a cycle"));
            }
            current = parents[parent];
        }
    }
    Ok(())
}

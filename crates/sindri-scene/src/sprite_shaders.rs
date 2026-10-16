//! The sprite shaders a scene names, and the checked shaders a host bound.
//!
//! Kept beside the texture bindings because they arrive the same way — a
//! host loads a project file and binds it under the path a scene names — and
//! because every host already hands those bindings to extraction. A shader
//! nothing has bound draws its sprites with the built-in look, which is the
//! same courtesy a missing texture gets: the frame draws, and the host says
//! which file it could not find.

use std::collections::{BTreeMap, BTreeSet};

use sindri_core::{SceneComponent, World};
use sindri_render::{SpriteShader, SpriteShaderError};

use crate::SpriteComponent;

/// Checked sprite shaders by the asset path a scene names them by.
#[derive(Clone, Debug, Default)]
pub struct SpriteShaderBindings {
    bound: BTreeMap<String, SpriteShader>,
}

impl SpriteShaderBindings {
    /// Checks `source` and binds it under `path`, replacing any earlier load.
    ///
    /// A source that does not compile is refused with the line it fails on,
    /// and the earlier load is dropped too: a scene showing the last good
    /// version of a file that is now broken hides the break from whoever is
    /// editing it, while the built-in look makes it plain that nothing of
    /// theirs is drawing.
    pub fn bind(&mut self, path: &str, source: &str) -> Result<(), SpriteShaderError> {
        match SpriteShader::new(path, source) {
            Ok(shader) => {
                self.bound.insert(path.to_owned(), shader);
                Ok(())
            }
            Err(error) => {
                self.bound.remove(path);
                Err(error)
            }
        }
    }

    /// Forgets the shader at `path`.
    pub fn unbind(&mut self, path: &str) -> Option<SpriteShader> {
        self.bound.remove(path)
    }

    /// The shader bound at `path`, if one is.
    #[must_use]
    pub fn get(&self, path: &str) -> Option<&SpriteShader> {
        self.bound.get(path)
    }

    /// Every bound path, sorted.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.bound.keys().map(String::as_str)
    }
}

/// Every built-in component that names a sprite shader.
pub const SHADER_NAMING_COMPONENTS: &[&str] = &[SpriteComponent::TYPE_NAME];

/// Every sprite shader a world draws with, which is the list hosts load from.
pub fn referenced_sprite_shaders(world: &World) -> BTreeSet<String> {
    world
        .entities()
        .flat_map(|(_, data)| {
            SHADER_NAMING_COMPONENTS
                .iter()
                .filter_map(|type_name| data.components.get(*type_name))
        })
        .filter_map(|payload| payload.get("shader"))
        .filter_map(serde_json::Value::as_str)
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Every sprite shader a world names that nothing has bound.
pub fn unresolved_sprite_shaders(
    world: &World,
    bindings: &SpriteShaderBindings,
) -> BTreeSet<String> {
    referenced_sprite_shaders(world)
        .into_iter()
        .filter(|path| bindings.get(path).is_none())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str =
        "fn sprite_shader(input: SpriteShaderInput) -> vec4<f32> { return input.color; }\n";

    #[test]
    fn a_valid_shader_binds_under_its_path() {
        let mut bindings = SpriteShaderBindings::default();
        bindings.bind("shaders/a.wgsl", VALID).unwrap();
        assert_eq!(
            bindings.get("shaders/a.wgsl").unwrap().name(),
            "shaders/a.wgsl"
        );
    }

    #[test]
    fn a_broken_reload_drops_the_last_good_version() {
        let mut bindings = SpriteShaderBindings::default();
        bindings.bind("shaders/a.wgsl", VALID).unwrap();
        assert!(bindings.bind("shaders/a.wgsl", "fn broken(").is_err());
        assert!(bindings.get("shaders/a.wgsl").is_none());
    }

    #[test]
    fn a_world_names_its_shaders_once_each() {
        let mut world = World::default();
        for shader in ["shaders/a.wgsl", "shaders/a.wgsl", ""] {
            world.spawn(sindri_core::EntityData {
                components: [(
                    SpriteComponent::TYPE_NAME.to_owned(),
                    serde_json::json!({ "texture": "t.png", "shader": shader }),
                )]
                .into(),
                ..sindri_core::EntityData::default()
            });
        }
        let names: Vec<_> = referenced_sprite_shaders(&world).into_iter().collect();
        assert_eq!(names, ["shaders/a.wgsl"]);
        assert_eq!(
            unresolved_sprite_shaders(&world, &SpriteShaderBindings::default()).len(),
            1
        );
    }
}

//! What can stop a run.

use sindri_platform::AudioError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error(transparent)]
    Scene(#[from] sindri_scene::SceneExtractError),
    #[error(transparent)]
    Physics(#[from] sindri_scene::PhysicsSyncError),
    #[error(transparent)]
    SceneSwitch(#[from] sindri_core::SceneSwitchError),
    #[error("no scene named '{0}' is in this project")]
    UnknownScene(String),
    #[error(transparent)]
    Component(#[from] sindri_core::ComponentRegistryError),
    #[error(transparent)]
    TileSet(#[from] sindri_core::TileSetError),
    #[error(transparent)]
    Animation(#[from] sindri_scene::AnimationError),
    #[error(transparent)]
    Audio(#[from] AudioError),
    #[error("presentation could not be composed: {0}")]
    Weave(String),
    /// One of the engine's own assets would not decode: a broken build.
    #[error(transparent)]
    Builtin(#[from] Box<sindri_assets::BuiltinError>),
}

impl From<sindri_assets::BuiltinError> for RuntimeError {
    fn from(error: sindri_assets::BuiltinError) -> Self {
        Self::Builtin(Box::new(error))
    }
}

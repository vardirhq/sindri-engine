//! What can stop Gather, on either host.

use sindri_platform::{AudioError, HostError};
use sindri_render::{FrameEncodeError, TextureError};
use sindri_scene::SheetBindError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CausewayError {
    /// The world is built rather than authored, so it can fail on its own.
    #[error("the world could not be generated: {0}")]
    Generated(String),
    #[error(transparent)]
    Scene(#[from] sindri_scene::SceneExtractError),
    #[error(transparent)]
    Physics(#[from] sindri_scene::PhysicsSyncError),
    #[error("the manifest records no scene, so there is nothing to open")]
    MissingScene,
    #[error(transparent)]
    SceneSwitch(#[from] sindri_core::SceneSwitchError),
    #[error("no scene named '{0}' is in this project")]
    UnknownScene(String),
    #[error(transparent)]
    Document(#[from] sindri_core::SceneError),
    #[error(transparent)]
    World(#[from] sindri_core::WorldError),
    #[error(transparent)]
    Component(#[from] sindri_core::ComponentRegistryError),
    #[error(transparent)]
    Asset(#[from] sindri_core::AssetIdError),
    #[error(transparent)]
    Sheet(#[from] sindri_core::SheetError),
    #[error(transparent)]
    SheetBind(#[from] SheetBindError),
    #[error(transparent)]
    TileSet(#[from] sindri_core::TileSetError),
    #[error(transparent)]
    Prefab(#[from] sindri_core::PrefabJsonError),
    #[error(transparent)]
    Decode(#[from] sindri_assets::AssetDecodeError),
    #[error(transparent)]
    Audio(#[from] AudioError),
    #[error(transparent)]
    Texture(#[from] TextureError),
    #[error(transparent)]
    Model(#[from] sindri_render::ModelRenderError),
    #[error(transparent)]
    Animation(#[from] sindri_scene::AnimationError),
    #[error(transparent)]
    Json(#[from] sindri_core::SceneJsonError),
    #[error(transparent)]
    Frame(#[from] FrameEncodeError),
    #[error("Gather presentation could not be composed: {0}")]
    Weave(String),
    /// The player's own work: the engine's built-in textures.
    #[error(transparent)]
    Player(#[from] Box<sindri_player::PlayerError>),
    /// The run itself: a script's scene change, a solver, a stylesheet.
    #[error(transparent)]
    Runtime(#[from] sindri_runtime::RuntimeError),
    #[error(transparent)]
    Host(#[from] Box<HostError<sindri_runtime::RuntimeError>>),
}

impl From<HostError<sindri_runtime::RuntimeError>> for CausewayError {
    fn from(error: HostError<sindri_runtime::RuntimeError>) -> Self {
        Self::Host(Box::new(error))
    }
}

impl From<sindri_player::PlayerError> for CausewayError {
    fn from(error: sindri_player::PlayerError) -> Self {
        Self::Player(Box::new(error))
    }
}

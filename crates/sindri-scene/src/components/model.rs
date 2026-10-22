//! `sindri.model`: an external imported model, never inline scene geometry.

use serde::Deserialize;
use sindri_core::{AssetId, SceneComponent};

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ModelComponent {
    pub asset: AssetId,
    #[serde(default)]
    pub layer: i32,
    /// Flat RGBA silhouette visible only behind opaque non-subject geometry.
    #[serde(default)]
    pub occluded_color: Option<[f32; 4]>,
}

impl SceneComponent for ModelComponent {
    const TYPE_NAME: &'static str = "sindri.model";
}

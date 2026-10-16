//! Standard numeric material overrides; asset textures and alpha modes stay owned.

use serde::Deserialize;
use sindri_core::SceneComponent;
use sindri_render::{MaterialError, MaterialFactors};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(try_from = "Document")]
pub struct MaterialComponent {
    pub factors: MaterialFactors,
}

#[derive(Deserialize)]
#[serde(default)]
struct Document {
    base_color: [f32; 4],
    metallic: f32,
    roughness: f32,
    emissive: [f32; 3],
}

impl Default for Document {
    fn default() -> Self {
        let factors = MaterialFactors::default();
        Self {
            base_color: factors.base_color,
            metallic: factors.metallic,
            roughness: factors.roughness,
            emissive: factors.emissive,
        }
    }
}

impl TryFrom<Document> for MaterialComponent {
    type Error = MaterialError;

    fn try_from(value: Document) -> Result<Self, Self::Error> {
        let factors = MaterialFactors {
            base_color: value.base_color,
            metallic: value.metallic,
            roughness: value.roughness,
            emissive: value.emissive,
        };
        factors.validate()?;
        Ok(Self { factors })
    }
}

impl SceneComponent for MaterialComponent {
    const TYPE_NAME: &'static str = "sindri.material";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_hdr_emission_are_checked_without_clamping() {
        let component: MaterialComponent = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(component.factors, MaterialFactors::default());
        let component: MaterialComponent =
            serde_json::from_value(serde_json::json!({"emissive": [2.0, 0.1, 0.0]})).unwrap();
        assert!((component.factors.emissive[0] - 2.0).abs() < f32::EPSILON);
        for invalid in [
            serde_json::json!({"metallic": 1.1}),
            serde_json::json!({"roughness": -0.1}),
            serde_json::json!({"base_color": [1.0, 1.0, 1.0, 2.0]}),
            serde_json::json!({"emissive": [-1.0, 0.0, 0.0]}),
            serde_json::json!({"emissive": [1e99, 0.0, 0.0]}),
        ] {
            assert!(serde_json::from_value::<MaterialComponent>(invalid).is_err());
        }
    }
}

//! Standard material overrides; asset textures and alpha modes stay owned.

use serde::Deserialize;
use sindri_core::SceneComponent;
use sindri_render::{MaterialFactors, MaterialSidedness};

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(try_from = "Document")]
pub struct MaterialComponent {
    pub profile: String,
    pub override_factors: bool,
    pub override_sidedness: bool,
    pub factors: MaterialFactors,
    pub sidedness: MaterialSidedness,
}

#[derive(Deserialize)]
#[serde(default)]
struct Document {
    profile: String,
    override_factors: bool,
    override_sidedness: bool,
    base_color: [f32; 4],
    metallic: f32,
    roughness: f32,
    emissive: [f32; 3],
    sidedness: SidednessDocument,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SidednessDocument {
    #[default]
    Inherit,
    Single,
    Double,
}

impl From<SidednessDocument> for MaterialSidedness {
    fn from(value: SidednessDocument) -> Self {
        match value {
            SidednessDocument::Inherit => Self::Inherit,
            SidednessDocument::Single => Self::Single,
            SidednessDocument::Double => Self::Double,
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        let factors = MaterialFactors::default();
        Self {
            profile: String::new(),
            override_factors: false,
            override_sidedness: false,
            base_color: factors.base_color,
            metallic: factors.metallic,
            roughness: factors.roughness,
            emissive: factors.emissive,
            sidedness: SidednessDocument::Inherit,
        }
    }
}

impl TryFrom<Document> for MaterialComponent {
    type Error = crate::RenderMaterialError;

    fn try_from(value: Document) -> Result<Self, Self::Error> {
        let factors = MaterialFactors {
            base_color: value.base_color,
            metallic: value.metallic,
            roughness: value.roughness,
            emissive: value.emissive,
        };
        factors.validate()?;
        crate::render_material::check_reference(&value.profile)?;
        Ok(Self {
            profile: value.profile,
            override_factors: value.override_factors,
            override_sidedness: value.override_sidedness,
            factors,
            sidedness: value.sidedness.into(),
        })
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
        assert_eq!(component.sidedness, MaterialSidedness::Inherit);
        let component: MaterialComponent =
            serde_json::from_value(serde_json::json!({"emissive": [2.0, 0.1, 0.0]})).unwrap();
        assert!((component.factors.emissive[0] - 2.0).abs() < f32::EPSILON);
        for invalid in [
            serde_json::json!({"metallic": 1.1}),
            serde_json::json!({"roughness": -0.1}),
            serde_json::json!({"base_color": [1.0, 1.0, 1.0, 2.0]}),
            serde_json::json!({"emissive": [-1.0, 0.0, 0.0]}),
            serde_json::json!({"emissive": [1e99, 0.0, 0.0]}),
            serde_json::json!({"sidedness": "both"}),
            serde_json::json!({"sidedness": true}),
        ] {
            assert!(serde_json::from_value::<MaterialComponent>(invalid).is_err());
        }
    }

    #[test]
    fn sidedness_is_explicit_and_independent_of_numeric_factors() {
        for (name, expected) in [
            ("inherit", MaterialSidedness::Inherit),
            ("single", MaterialSidedness::Single),
            ("double", MaterialSidedness::Double),
        ] {
            let component: MaterialComponent =
                serde_json::from_value(serde_json::json!({"sidedness": name})).unwrap();
            assert_eq!(component.sidedness, expected);
            assert_eq!(component.factors, MaterialFactors::default());
        }
    }
}

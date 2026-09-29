//! Reusable authored data that is not an entity.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const PROFILE_FORMAT_VERSION: u32 = 1;
pub const PROFILE_SUFFIX: &str = ".profile";
pub const LEGACY_PROFILE_SUFFIX: &str = ".profile.json";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ProfileDocument {
    pub format_version: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, rename = "type", skip_serializing_if = "String::is_empty")]
    pub profile_type: String,
    #[serde(default)]
    pub values: BTreeMap<String, Value>,
}

impl Default for ProfileDocument {
    fn default() -> Self {
        Self { format_version: PROFILE_FORMAT_VERSION, name: String::new(), profile_type: String::new(), values: BTreeMap::new() }
    }
}

impl ProfileDocument {
    pub fn from_json(json: &str) -> Result<Self, ProfileError> {
        let document: Self = serde_json::from_str(json)?;
        document.validate()?;
        Ok(document)
    }

    pub fn to_canonical_json(&self) -> Result<String, ProfileError> {
        self.validate()?;
        let mut json = serde_json::to_string_pretty(self)?;
        json.push('\n');
        Ok(json)
    }

    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.format_version != PROFILE_FORMAT_VERSION {
            return Err(ProfileError::UnsupportedVersion { found: self.format_version, supported: PROFILE_FORMAT_VERSION });
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("profile format {found} is unsupported; this runtime supports {supported}")]
    UnsupportedVersion { found: u32, supported: u32 },
}

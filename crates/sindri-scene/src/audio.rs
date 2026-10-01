use serde::{Deserialize, Serialize};
use sindri_core::SceneComponent;

fn default_volume() -> f32 {
    1.0
}

/// Authored audio attached to a scene entity.
///
/// The component only describes what should play. Device state, active voices,
/// and browser unlock state are runtime concerns and never serialize into a
/// scene file.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AudioSourceComponent {
    /// Logical project asset ID, for example `audio/music.ogg`.
    pub clip: String,
    /// Start this source when the scene/game starts.
    #[serde(default)]
    pub autoplay: bool,
    /// Repeat until stopped rather than playing once.
    #[serde(default)]
    pub looping: bool,
    /// Linear gain in the inclusive 0..=1 range.
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// The bus it plays through. Left empty, a looping source is music and a
    /// one-shot an effect, as `Audio.loop` and `Audio.play` route.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bus: String,
}

impl AudioSourceComponent {
    #[must_use]
    pub fn normalized_volume(&self) -> f32 {
        self.volume.clamp(0.0, 1.0)
    }

    /// The bus this source plays through, with the default filled in.
    #[must_use]
    pub fn bus(&self) -> &str {
        match (self.bus.as_str(), self.looping) {
            ("", true) => "music",
            ("", false) => "effects",
            (named, _) => named,
        }
    }
}

impl SceneComponent for AudioSourceComponent {
    const TYPE_NAME: &'static str = "sindri.audio.source";
}

#[cfg(test)]
mod tests {
    use sindri_core::SceneComponent;

    use super::AudioSourceComponent;

    #[test]
    fn audio_source_has_the_canonical_component_name() {
        assert_eq!(AudioSourceComponent::TYPE_NAME, "sindri.audio.source");
    }

    #[test]
    fn missing_optional_fields_have_safe_defaults() {
        let source: AudioSourceComponent = serde_json::from_value(serde_json::json!({
            "clip": "audio/pickup.wav"
        }))
        .expect("audio component");
        assert!(!source.autoplay);
        assert!(!source.looping);
        assert!((source.volume - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn runtime_volume_is_bounded() {
        let mut source = AudioSourceComponent {
            clip: "audio/music.ogg".to_owned(),
            autoplay: true,
            looping: true,
            volume: 4.0,
            bus: String::new(),
        };
        assert_eq!(source.bus(), "music");
        assert!((source.normalized_volume() - 1.0).abs() < f32::EPSILON);

        source.volume = -2.0;
        assert!(source.normalized_volume().abs() < f32::EPSILON);
    }
}

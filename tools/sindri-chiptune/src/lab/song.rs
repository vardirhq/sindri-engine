//! `.sindrisong`: everything needed to reopen, re-edit and re-render one score.
use super::{Mood, Note, Settings, instruments::Rack, midi};
use serde::{Deserialize, Serialize};

/// The `.sindrisong` document version this build writes and reads.
pub(super) const FORMAT: u32 = 1;
pub(super) const MAX_SNAPSHOTS: usize = 16;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NamedSnapshot {
    pub(super) name: String,
    pub(super) song: SongFile,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SongFile {
    pub(super) format: u32,
    pub(super) seed: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) seed_text: Option<String>,
    pub(super) bpm: f64,
    pub(super) key: i32,
    pub(super) energy: f64,
    pub(super) mood: Mood,
    pub(super) rack: Rack,
    /// Bass, arpeggio and lead, as edited; the percussion voice is generated.
    pub(super) tracks: [Vec<Note>; 3],
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) snapshots: Vec<NamedSnapshot>,
}

impl SongFile {
    pub(super) fn new(settings: &Settings, tracks: &[Vec<Note>; 3]) -> Self {
        Self {
            format: FORMAT,
            seed: settings.seed,
            seed_text: settings.seed_text.clone(),
            bpm: settings.bpm,
            key: settings.key,
            energy: settings.energy,
            mood: settings.mood,
            rack: settings.rack.clone(),
            tracks: tracks.clone(),
            snapshots: Vec::new(),
        }
    }

    /// The render settings this song was saved with.
    pub(super) fn settings(&self) -> Settings {
        Settings {
            seed: self.seed,
            seed_text: self.seed_text.clone(),
            bpm: self.bpm,
            key: self.key,
            energy: self.energy,
            mood: self.mood,
            rack: self.rack.clone(),
            ..Settings::default()
        }
    }

    pub(super) fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a song serializes") + "\n"
    }

    pub(super) fn from_json(text: &str) -> Result<Self, String> {
        let song: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        song.validate()?;
        if song.snapshots.len() > MAX_SNAPSHOTS {
            return Err("A song can contain at most 16 snapshots.".into());
        }
        for snapshot in &song.snapshots {
            if snapshot.name.trim().is_empty()
                || snapshot.name.chars().count() > 64
                || snapshot.name.chars().any(char::is_control)
                || !snapshot.song.snapshots.is_empty()
            {
                return Err("A snapshot needs a short single-line name and cannot contain nested snapshots.".into());
            }
            snapshot.song.validate()?;
        }
        Ok(song)
    }

    fn validate(&self) -> Result<(), String> {
        let song = self;
        if let Some(text) = &song.seed_text
            && super::seed::parse(text)?.value != song.seed
        {
            return Err("The saved seed phrase does not match its numeric seed.".into());
        }
        if song.format != FORMAT {
            return Err(format!("unsupported .sindrisong format {}", song.format));
        }
        if !(60.0..=220.0).contains(&song.bpm)
            || !(0.0..=1.0).contains(&song.energy)
            || !(0..=11).contains(&song.key)
        {
            return Err("bpm must be 60..220, energy 0..1 and key 0..11".into());
        }
        if song
            .rack
            .tracks
            .iter()
            .any(|track| !(0.0..=1.0).contains(&track.volume) || !(-2..=2).contains(&track.octave))
        {
            return Err("a mixer level must be 0..1 and an octave shift -2..2".into());
        }
        song.rack.arrangement.validate()?;
        if !(0.0..=1.0).contains(&song.rack.pad.volume) {
            return Err("A chord level must be 0..1.".into());
        }
        midi::validate(&song.tracks)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::compose_with;

    #[test]
    fn a_song_round_trips_with_its_edits_and_mixer() {
        let mut settings = Settings {
            seed: 77,
            mood: Mood::Hopeful,
            ..Settings::default()
        };
        settings.rack.tracks[1].muted = true;
        settings.rack.tracks[2].octave = 1;
        let mut tracks = compose_with(&settings);
        tracks[2][0].velocity = 127;
        let song = SongFile::new(&settings, &tracks);
        let reopened = SongFile::from_json(&song.to_json()).expect("round trip");
        assert_eq!(reopened, song);
        assert_eq!(reopened.settings().rack, settings.rack);
        assert_eq!(reopened.tracks, tracks);
    }

    #[test]
    fn a_damaged_song_is_refused_with_a_reason() {
        let settings = Settings::default();
        let song = SongFile::new(&settings, &compose_with(&settings));
        let mut overlapping = song.clone();
        overlapping.tracks[2][0].len = crate::lab::STEPS;
        assert!(SongFile::from_json(&overlapping.to_json()).is_err());
        let mut fast = song;
        fast.bpm = 999.0;
        assert!(SongFile::from_json(&fast.to_json()).is_err());
        assert!(SongFile::from_json("{}").is_err());
    }
}

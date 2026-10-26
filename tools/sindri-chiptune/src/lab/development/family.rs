use super::{nearest_class, scale_pitch, scale_position};
use crate::lab::{Mood, Note, ensemble, song::SongFile};

pub(in crate::lab) const FAMILY_NAMES: [&str; 3] = ["Exploration", "Danger", "Victory"];

// Bounded musical dynamics are rounded and clamped before conversion.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn dynamics(value: u8, factor: f64) -> u8 {
    (f64::from(value) * factor).round().clamp(1.0, 127.0) as u8
}

/// Related cues retain note timings and diatonic melodic contours, with new colors.
pub(in crate::lab) fn family(source: &SongFile) -> [SongFile; 3] {
    let profiles = [
        (Mood::Mysterious, 0.85, 0.65, 0.78),
        (Mood::Tense, 1.15, 1.35, 1.16),
        (Mood::Hopeful, 1.05, 1.15, 1.08),
    ];
    std::array::from_fn(|index| {
        let (mood, tempo, energy, dynamic) = profiles[index];
        let mut settings = source.settings();
        settings.bpm = (settings.bpm * tempo).clamp(60.0, 220.0);
        let minimum = match index {
            1 => 0.7,
            2 => 0.6,
            _ => 0.05,
        };
        settings.energy = (settings.energy * energy).clamp(minimum, 1.0);
        settings.mood = mood;
        let palette = ensemble::generate(&settings);
        for layer in 0..5 {
            settings.rack.channel_mut(layer).preset = palette.channel(layer).preset;
        }
        let mut tracks = source.tracks.clone();
        for track in &mut tracks {
            for note in track {
                let (degree, chromatic) = scale_position(note.pitch, source.key, source.mood);
                note.pitch = (scale_pitch(degree, source.key, mood) + chromatic).clamp(0, 127);
                note.velocity = dynamics(note.velocity, dynamic);
            }
        }
        if index == 2 {
            resolve(&mut tracks[0], settings.key);
            resolve(&mut tracks[2], settings.key);
        }
        SongFile::new(&settings, &tracks)
    })
}

fn resolve(track: &mut [Note], key: i32) {
    if let Some(last) = track.last_mut() {
        last.pitch = nearest_class(last.pitch, key);
    }
}

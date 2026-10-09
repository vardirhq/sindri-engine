//! Deterministic kick, backbeat, hats and phrase fills for the fourth chip voice.
use crate::desktop::{Mood, RATE, STEPS, instruments::Preset};
use std::f64::consts::TAU;

// A 32-bar timeline bounds indices, and hash arithmetic intentionally wraps.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(super) fn sample(
    seed: u64,
    time: f64,
    step: f64,
    energy: f64,
    mood: Mood,
    preset: Preset,
) -> f64 {
    let tick = (time / step).floor() as usize;
    if tick >= STEPS {
        return 0.0;
    }
    let bar = tick / 16;
    let slot = tick % 16;
    if !(4..30).contains(&bar) {
        return 0.0;
    }
    let age = time - f64::from(u32::try_from(tick).expect("step")) * step;
    let mut hash = (time * f64::from(RATE)) as u64 ^ seed ^ 0xabcd_1234;
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^= hash >> 33;
    let white = if hash & 1 == 0 { -1.0 } else { 1.0 };
    let sparse = matches!(mood, Mood::Mysterious | Mood::Melancholic);
    let kick = slot == 0 || !sparse && slot == 8 || mood == Mood::Tense && slot == 10;
    let snare = slot == 12 || !sparse && slot == 4;
    let hat = !sparse && slot.is_multiple_of(2) || sparse && slot == 6;
    let fill = bar % 4 == 3 && slot >= 14 && bar >= 8;
    let section = if bar < 8 {
        0.5
    } else if bar >= 24 {
        0.6
    } else {
        1.0
    };
    let mut sound = 0.0;
    if kick && age < 0.16 {
        // Integrate an exponential pitch drop for a soft pitched chip kick.
        let phase = 48.0 * age + 90.0 / 35.0 * (1.0 - (-35.0 * age).exp());
        sound += (TAU * phase).sin() * (-28.0 * age).exp() * 0.75;
    }
    if snare || fill {
        sound += preset.noise(white, age) * if fill { 0.42 } else { 0.6 };
    } else if hat && age < 0.035 {
        sound += white * (-120.0 * age).exp() * 0.16;
    }
    sound * section * (energy / 0.45).min(1.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn intro_and_final_bars_leave_space() {
        for tick in [0, 12, 30 * 16, 31 * 16 + 12] {
            let time = f64::from(tick) * 0.1 + 0.01;
            assert!(
                sample(42, time, 0.1, 0.9, Mood::Tense, Preset::CrispNoise).abs() < f64::EPSILON
            );
        }
    }
    #[test]
    fn kick_and_backbeat_have_different_textures() {
        let kick = sample(42, 12.8 + 0.01, 0.1, 0.5, Mood::Hopeful, Preset::CrispNoise);
        let snare = sample(42, 13.2 + 0.01, 0.1, 0.5, Mood::Hopeful, Preset::CrispNoise);
        assert!((kick - snare).abs() > 0.01);
    }
}

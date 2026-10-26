//! Smooth layer entrances without cutting effect tails or changing legacy mixes.
use super::Engine;

impl Engine {
    pub(super) fn participation(&mut self) -> [f64; 5] {
        let section = (self.step_index() / 128).min(3);
        for (layer, gain) in self.activity.iter_mut().enumerate() {
            let target = if self.rack.arrangement.active(section, layer) {
                1.0
            } else {
                0.0
            };
            // About 34 ms to 95%: section changes and live edits do not click.
            *gain += (target - *gain) * 0.002;
        }
        self.activity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::{Settings, compose_with};

    #[test]
    fn resting_layers_do_not_leak_audio_or_duck_the_other_parts() {
        let mut settings = Settings::default();
        settings.rack.arrangement.sections = [0; 4];
        let mut engine = Engine::new(settings.clone(), compose_with(&settings));
        let mut out = vec![[0.0; 2]; 4096];
        engine.render(&mut out, false);
        assert!(out.iter().flatten().all(|s| s.abs() < f32::EPSILON));
        assert!(engine.activity.iter().all(|v| v.abs() < f64::EPSILON));
    }

    #[test]
    fn participation_changes_ramp_and_legacy_layers_stay_at_unity() {
        let settings = Settings::default();
        let mut engine = Engine::new(settings.clone(), compose_with(&settings));
        assert!(
            engine
                .participation()
                .iter()
                .all(|v| (*v - 1.0).abs() < f64::EPSILON)
        );
        engine.rack.arrangement.sections = [0; 4];
        let first = engine.participation();
        assert!(first.iter().all(|v| *v > 0.99 && *v < 1.0));
        for _ in 0..2000 {
            engine.participation();
        }
        assert!(engine.activity.iter().all(|v| *v < 0.02));
    }
}

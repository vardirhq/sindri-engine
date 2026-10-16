use super::*;
use crate::lab::{Mood, compose_with};

fn engine(mood: Mood) -> Engine {
    let settings = Settings {
        mood,
        energy: 0.8,
        ..Settings::default()
    };
    let tracks = compose_with(&settings);
    Engine::new(settings, tracks)
}

fn render(engine: &mut Engine, seconds: f64) -> Vec<[f32; 2]> {
    let mut out = vec![[0.0; 2]; usize::try_from(samples(seconds)).expect("short")];
    engine.render(&mut out, true);
    out
}

#[test]
fn rendering_is_deterministic() {
    let mut a = engine(Mood::Hopeful);
    let mut b = engine(Mood::Hopeful);
    a.seek_bar(8);
    b.seek_bar(8);
    assert_eq!(render(&mut a, 1.0), render(&mut b, 1.0));
}

#[test]
fn every_mood_fills_the_stereo_field_without_clipping() {
    for mood in [
        Mood::Hopeful,
        Mood::Tense,
        Mood::Mysterious,
        Mood::Melancholic,
    ] {
        let mut e = engine(mood);
        e.seek_bar(16);
        let out = render(&mut e, 3.0);
        let peak = out.iter().flatten().fold(0.0_f32, |a, b| a.max(b.abs()));
        let side: f32 = out.iter().map(|[l, r]| (l - r).abs()).sum::<f32>();
        let mid: f32 = out.iter().map(|[l, r]| (l + r).abs()).sum::<f32>();
        assert!(peak > 0.3 && peak < 0.95, "{mood:?} peak {peak}");
        assert!(side > mid * 0.05, "{mood:?} is too narrow");
        assert!(out.iter().flatten().all(|s| s.is_finite()));
    }
}

#[test]
fn a_song_that_does_not_loop_rings_out_then_stops() {
    let mut e = engine(Mood::Tense);
    e.seek(e.length() - samples(0.5));
    let mut out = vec![[0.0; 2]; usize::try_from(samples(10.0)).expect("short")];
    let written = e.render(&mut out, false);
    assert!(written > usize::try_from(samples(0.5)).expect("short"));
    assert!(written < out.len(), "it stopped");
}

#[test]
fn the_loop_point_carries_its_tails() {
    // Played round, the first beat of the song still has the end's room in it.
    let mut e = engine(Mood::Melancholic);
    e.seek(e.length() - samples(1.0));
    let mut around = vec![[0.0; 2]; usize::try_from(samples(1.05)).expect("short")];
    e.render(&mut around, true);
    let seam = &around[usize::try_from(samples(1.0)).expect("short")..];
    assert!(
        seam.iter().flatten().any(|s| s.abs() > 1e-4),
        "silence at the seam"
    );
}

#[test]
fn edits_and_tempo_change_what_plays() {
    let mut e = engine(Mood::Hopeful);
    e.seek_bar(8);
    let before = render(&mut e, 0.5);
    let mut e = engine(Mood::Hopeful);
    e.set_tracks(std::array::from_fn(|_| Vec::new()));
    e.seek_bar(8);
    let without = render(&mut e, 0.5);
    assert_ne!(before, without);
}

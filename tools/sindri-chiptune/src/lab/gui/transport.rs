//! Playback: the live song, one-shot effects, and the work that arrives later.
use super::{ComposerApp, Speaker, platform::Opened};
use crate::lab::live::{Command, LiveSource};
use eframe::egui;
use std::{
    sync::mpsc::{self, Receiver},
    time::Duration,
};

impl ComposerApp {
    /// The speaker, opened on first use so a window that never plays never asks.
    fn speaker(&mut self) -> Option<&mut Speaker> {
        if self.speaker.is_none() {
            match Speaker::open() {
                Ok(speaker) => self.speaker = Some(speaker),
                Err(error) => {
                    self.status = format!("Audio device unavailable: {error}");
                    return None;
                }
            }
        }
        self.speaker.as_mut()
    }

    pub(super) fn start_live(&mut self) {
        let settings = self.selected_settings();
        let notes = self.scores[self.selected].clone();
        if let Some(transport) = &self.transport {
            let _ = transport.tx.send(Command::Replace(settings.clone(), notes));
            if let Some(speaker) = &self.speaker {
                speaker.set_paused(false);
            }
        } else {
            let looping = self.looping;
            let Some(speaker) = self.speaker() else {
                return;
            };
            let (source, transport) = LiveSource::new(settings.clone(), notes);
            let _ = transport.tx.send(Command::Loop(looping));
            speaker.live(source);
            self.transport = Some(transport);
        }
        self.paused = false;
        self.playing = Some(settings.clone());
        self.last_sent = Some(settings);
        self.status = "Playing selected score. Mixer and tempo changes are live.".into();
    }

    pub(super) fn stop(&mut self) {
        if let Some(speaker) = &mut self.speaker {
            speaker.stop_live();
        }
        self.transport = None;
        self.playing = None;
        self.last_sent = None;
        self.paused = false;
        self.status = "Stopped.".into();
    }

    pub(super) fn toggle_play(&mut self) {
        if self.transport.is_some() {
            self.paused = !self.paused;
            if let Some(speaker) = &self.speaker {
                speaker.set_paused(self.paused);
            }
        } else {
            self.start_live();
        }
    }

    /// Plays the current effect from the start.
    pub(super) fn audition(&mut self) {
        self.sfx.refresh();
        let samples = self.sfx.samples.clone();
        if let Some(speaker) = self.speaker() {
            speaker.once(&samples);
            self.sfx.started = Some(web_time::Instant::now());
        }
    }

    pub(super) fn sync_audio(&mut self, ui: &egui::Ui) {
        if let (Some(transport), Some(playing)) = (&self.transport, &mut self.playing) {
            playing.rack = self.settings.rack.clone();
            playing.bpm = self.settings.bpm;
            if self.last_sent.as_ref() != Some(playing) {
                let _ = transport.tx.send(Command::Update(playing.clone()));
                self.last_sent = Some(playing.clone());
            }
            ui.ctx().request_repaint_after(Duration::from_millis(33));
        }
    }

    pub(super) fn playhead_bar(&self) -> f32 {
        self.transport.as_ref().map_or(0.0, |transport| {
            let samples = transport
                .position
                .load(std::sync::atomic::Ordering::Relaxed);
            let samples = u32::try_from(samples).unwrap_or(u32::MAX);
            // A 32-bar song has fewer than 6 million samples; display precision suffices.
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            let bar = (f64::from(samples)
                / (f64::from(crate::lab::RATE) * 240.0 / self.settings.bpm))
                as f32;
            bar.min(31.999)
        })
    }

    /// Takes whatever finished since the last frame: a song, an export, a file.
    pub(super) fn poll(&mut self, ui: &egui::Ui) {
        if self.speaker.as_ref().is_some_and(Speaker::live_finished) && !self.paused {
            self.stop();
            self.status = "Finished. Press Play to audition again.".into();
        }
        match self.job.as_ref().map(Receiver::try_recv) {
            Some(Ok(result)) => {
                self.job = None;
                self.status = match result {
                    Ok(path) => format!("Saved {path}"),
                    Err(e) => format!("Export failed: {e}"),
                };
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.job = None;
                self.status = "Export worker disconnected.".into();
            }
            Some(Err(mpsc::TryRecvError::Empty)) => {
                ui.ctx().request_repaint_after(Duration::from_millis(80));
            }
            None => {}
        }
        while let Some(opened) = self.files.poll() {
            self.opened(&opened);
        }
    }

    fn opened(&mut self, file: &Opened) {
        use super::platform::Purpose;
        match file.purpose {
            Purpose::Song => self.open_song(file),
            Purpose::Midi => self.import_midi(file),
            Purpose::Sfx => self.open_sfx(file),
        }
    }
}

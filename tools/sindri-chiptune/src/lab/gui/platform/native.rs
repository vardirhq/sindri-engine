//! The native host: rodio for sound, file dialogs and the filesystem for files.
use super::{Opened, Purpose, SaveAs};
use crate::lab::{RATE, live::LiveSource};
use rodio::{
    Player,
    buffer::SamplesBuffer,
    stream::{DeviceSinkBuilder, MixerDeviceSink},
};
use std::{fs, num::NonZero, path::PathBuf};

pub(in crate::lab::gui) struct Speaker {
    stream: MixerDeviceSink,
    live: Option<Player>,
    once: Option<Player>,
}

impl Speaker {
    pub(in crate::lab::gui) fn open() -> Result<Self, String> {
        let stream = DeviceSinkBuilder::open_default_sink().map_err(|e| e.to_string())?;
        Ok(Self {
            stream,
            live: None,
            once: None,
        })
    }

    /// Plays `source` in place of whatever song was playing.
    pub(in crate::lab::gui) fn live(&mut self, source: LiveSource) {
        self.stop_live();
        let player = Player::connect_new(self.stream.mixer());
        player.append(source);
        player.play();
        self.live = Some(player);
    }

    pub(in crate::lab::gui) fn set_paused(&self, paused: bool) {
        if let Some(player) = &self.live {
            if paused {
                player.pause();
            } else {
                player.play();
            }
        }
    }

    pub(in crate::lab::gui) fn stop_live(&mut self) {
        if let Some(player) = self.live.take() {
            player.stop();
        }
    }

    /// Whether a song that does not loop has played to its end.
    pub(in crate::lab::gui) fn live_finished(&self) -> bool {
        self.live.as_ref().is_some_and(Player::empty)
    }

    /// Plays a short sound once, cutting off the previous one.
    pub(in crate::lab::gui) fn once(&mut self, samples: &[f32]) {
        let player = Player::connect_new(self.stream.mixer());
        player.append(SamplesBuffer::new(
            NonZero::new(1).expect("mono"),
            NonZero::new(RATE).expect("rate"),
            samples.to_vec(),
        ));
        player.play();
        self.once = Some(player);
    }
}

#[derive(Default)]
pub(in crate::lab::gui) struct Files {
    opened: Vec<Opened>,
}

impl Files {
    pub(in crate::lab::gui) fn open(
        &mut self,
        _context: &eframe::egui::Context,
        purpose: Purpose,
        label: &str,
        extensions: &[&str],
    ) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter(label, extensions)
            .pick_file()
        else {
            return;
        };
        // An unreadable file arrives empty and is refused, with its name, by
        // whatever parses it.
        let bytes = fs::read(&path).unwrap_or_default();
        self.opened.push(Opened {
            purpose,
            name: path.display().to_string(),
            bytes,
        });
    }

    pub(in crate::lab::gui) fn poll(&mut self) -> Option<Opened> {
        self.opened.pop()
    }

    /// Asks where, then writes. `None` when the person cancelled.
    // One signature for both hosts; neither keeps state for a save.
    #[allow(clippy::unused_self)]
    pub(in crate::lab::gui) fn save(
        &mut self,
        save: &SaveAs<'_>,
        bytes: &[u8],
    ) -> Result<Option<String>, String> {
        let Some(path) = Self::choose(save) else {
            return Ok(None);
        };
        fs::write(&path, bytes).map_err(|e| e.to_string())?;
        Ok(Some(path.display().to_string()))
    }

    pub(in crate::lab::gui) fn choose(save: &SaveAs<'_>) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .set_title(save.title)
            .set_file_name(save.file_name())
            .add_filter(save.label, &[save.extension])
            .save_file()
    }
}

//! Deliberately narrow Standard MIDI File interchange for three monophonic voices.
//! Supports format 0/1, metrical time, running status and constant-tempo 4/4.
use crate::desktop::{Note, STEPS, Settings};
use std::{error::Error, fs};

pub(super) fn validate(tracks: &[Vec<Note>; 3]) -> Result<(), &'static str> {
    for track in tracks {
        let mut end = 0;
        for note in track {
            if !(0..=127).contains(&note.pitch)
                || !(1..=127).contains(&note.velocity)
                || note.len == 0
                || note.start >= STEPS
                || note.len > STEPS - note.start
            {
                return Err("Notes must fit the 32-bar score and MIDI pitch/velocity range.");
            }
            if note.start < end {
                return Err("One voice per part: notes cannot overlap. Move or shorten the note.");
            }
            end = note.start + note.len;
        }
    }
    Ok(())
}
fn variable(mut value: u32, out: &mut Vec<u8>) {
    let mut bytes = [0_u8; 4];
    bytes[3] = u8::try_from(value & 127).expect("seven bits");
    let mut first = 3;
    while {
        value >>= 7;
        value != 0
    } {
        first -= 1;
        bytes[first] = u8::try_from(value & 127).expect("seven bits") | 128;
    }
    out.extend_from_slice(&bytes[first..]);
}
fn chunk(out: &mut Vec<u8>, tag: &[u8], data: &[u8]) {
    out.extend_from_slice(tag);
    out.extend_from_slice(
        &u32::try_from(data.len())
            .expect("small MIDI chunk")
            .to_be_bytes(),
    );
    out.extend_from_slice(data);
}
// Tempo is finite and bounded to 60..220 BPM before conversion to microseconds.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn encode(settings: &Settings, tracks: &[Vec<Note>; 3]) -> Result<Vec<u8>, &'static str> {
    validate(tracks)?;
    if !settings.bpm.is_finite() || !(60.0..=220.0).contains(&settings.bpm) {
        return Err("Unsupported tempo.");
    }
    let mut out = Vec::new();
    chunk(&mut out, b"MThd", &[0, 1, 0, 4, 1, 224]); // type 1, 4 tracks, 480 PPQ
    let tempo = (60_000_000.0 / settings.bpm).round() as u32;
    let t = tempo.to_be_bytes();
    chunk(
        &mut out,
        b"MTrk",
        &[
            0, 255, 81, 3, t[1], t[2], t[3], 0, 255, 88, 4, 4, 2, 24, 8, 0, 255, 47, 0,
        ],
    );
    for (channel, notes) in tracks.iter().enumerate() {
        let channel = u8::try_from(channel).expect("channel");
        let mut track = Vec::new();
        let mut last = 0;
        for note in notes {
            let start = u32::try_from(note.start).expect("step") * 120;
            variable(start - last, &mut track);
            track.extend_from_slice(&[
                0x90 | channel,
                u8::try_from(note.pitch).expect("pitch"),
                note.velocity,
            ]);
            let length = u32::try_from(note.len).expect("length") * 120;
            variable(length, &mut track);
            track.extend_from_slice(&[0x80 | channel, u8::try_from(note.pitch).expect("pitch"), 0]);
            last = start + length;
        }
        variable(
            u32::try_from(STEPS).expect("steps") * 120 - last,
            &mut track,
        );
        track.extend_from_slice(&[255, 47, 0]);
        chunk(&mut out, b"MTrk", &track);
    }
    Ok(out)
}
pub(super) fn write(
    path: &str,
    settings: &Settings,
    tracks: &[Vec<Note>; 3],
) -> Result<(), Box<dyn Error>> {
    fs::write(path, encode(settings, tracks)?)?;
    Ok(())
}

struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let bytes = self.0.get(..count).ok_or("Truncated MIDI file.")?;
        self.0 = &self.0[count..];
        Ok(bytes)
    }
    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn word(&mut self) -> Result<u16, String> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().expect("word")))
    }
    fn variable(&mut self) -> Result<u32, String> {
        let mut value = 0;
        for _ in 0..4 {
            let b = self.byte()?;
            value = (value << 7) | u32::from(b & 127);
            if b < 128 {
                return Ok(value);
            }
        }
        Err("Invalid MIDI variable-length quantity.".into())
    }
    fn chunk(&mut self, tag: &[u8]) -> Result<Reader<'a>, String> {
        if self.take(4)? != tag {
            return Err("Invalid MIDI chunk.".into());
        }
        let len = u32::from_be_bytes(self.take(4)?.try_into().expect("length"));
        Ok(Reader(self.take(
            usize::try_from(len).map_err(|_| "Chunk too large.")?,
        )?))
    }
}
type PendingNote = Option<(u64, u8)>;

struct Import {
    channels: [Vec<Note>; 16],
    active: Box<[[PendingNote; 128]]>,
    tempo: Option<u32>,
    ppq: u16,
}
impl Import {
    fn step(&self, ticks: u64) -> Result<usize, String> {
        let step = (ticks * 4 + u64::from(self.ppq) / 2) / u64::from(self.ppq);
        usize::try_from(step).map_err(|_| "MIDI timeline too long.".into())
    }
    fn note(&mut self, status: u8, pitch: u8, velocity: u8, ticks: u64) -> Result<(), String> {
        if pitch >= 128 || velocity >= 128 {
            return Err("Invalid MIDI data byte.".into());
        }
        let channel = usize::from(status & 15);
        let active = self.active[channel][usize::from(pitch)];
        if status & 0xf0 == 0x90 && velocity > 0 {
            if channel == 9 {
                return Err("Percussion import is unsupported; import tonal parts only.".into());
            }
            if active.is_some() {
                return Err("Overlapping identical MIDI notes are unsupported.".into());
            }
            self.active[channel][usize::from(pitch)] = Some((ticks, velocity));
        } else if let Some((start, level)) = active {
            let first = self.step(start)?;
            let end = self.step(ticks)?.max(first + 1);
            self.channels[channel].push(Note {
                pitch: i32::from(pitch),
                start: first,
                len: end - first,
                velocity: level,
            });
            self.active[channel][usize::from(pitch)] = None;
        }
        Ok(())
    }
    fn meta(&mut self, kind: u8, data: &[u8], tick: u64) -> Result<(), String> {
        match (kind, data) {
            (81, [a, b, c]) => {
                let tempo = u32::from_be_bytes([0, *a, *b, *c]);
                if tempo == 0
                    || self.tempo.is_some_and(|t| t != tempo)
                    || tick > 0 && self.tempo.is_none()
                {
                    return Err("Only constant-tempo MIDI is supported.".into());
                }
                self.tempo = Some(tempo);
            }
            (88, [4, 2, _, _]) => {}
            (88, _) => return Err("Only 4/4 MIDI is supported.".into()),
            _ => {}
        }
        Ok(())
    }
    fn track(&mut self, mut input: Reader<'_>) -> Result<(), String> {
        let mut tick = 0_u64;
        let mut running = 0;
        while !input.0.is_empty() {
            tick = tick
                .checked_add(u64::from(input.variable()?))
                .ok_or("Timeline overflow.")?;
            let first = input.byte()?;
            let status = if first < 128 {
                if running == 0 {
                    return Err("Missing MIDI running status.".into());
                }
                running
            } else {
                first
            };
            if status == 255 {
                running = 0;
                let kind = input.byte()?;
                let len =
                    usize::try_from(input.variable()?).map_err(|_| "Meta event too large.")?;
                self.meta(kind, input.take(len)?, tick)?;
                if kind == 47 {
                    break;
                }
            } else if status == 240 || status == 247 {
                running = 0;
                let len = usize::try_from(input.variable()?).map_err(|_| "SysEx too large.")?;
                input.take(len)?;
            } else if (128..240).contains(&status) {
                running = status;
                let a = if first < 128 { first } else { input.byte()? };
                let b = if status & 0xf0 == 0xc0 || status & 0xf0 == 0xd0 {
                    0
                } else {
                    input.byte()?
                };
                if a >= 128 || b >= 128 {
                    return Err("Invalid MIDI data byte.".into());
                }
                match status & 0xf0 {
                    0x80 | 0x90 => self.note(status, a, b, tick)?,
                    0xe0 => return Err("Pitch-bend import is unsupported.".into()),
                    0xb0 if a == 64 && b > 0 => {
                        return Err("Sustain-pedal import is unsupported.".into());
                    }
                    _ => {}
                }
            } else {
                return Err("Unsupported MIDI status byte.".into());
            }
        }
        Ok(())
    }
}
pub(super) fn read(bytes: &[u8]) -> Result<([Vec<Note>; 3], f64), String> {
    let mut input = Reader(bytes);
    let mut header = input.chunk(b"MThd")?;
    let format = header.word()?;
    let count = header.word()?;
    let ppq = header.word()?;
    if format > 1 || count == 0 || ppq == 0 || ppq & 0x8000 != 0 {
        return Err("Requires MIDI format 0/1 with metrical timing.".into());
    }
    let mut import = Import {
        channels: std::array::from_fn(|_| Vec::new()),
        active: vec![[None; 128]; 16].into_boxed_slice(),
        tempo: None,
        ppq,
    };
    for _ in 0..count {
        import.track(input.chunk(b"MTrk")?)?;
    }
    if import.active.iter().flatten().any(Option::is_some) {
        return Err("MIDI contains unterminated notes.".into());
    }
    let bpm = 60_000_000.0 / f64::from(import.tempo.unwrap_or(500_000));
    if !(60.0..=220.0).contains(&bpm) {
        return Err("MIDI tempo must be 60..220 BPM.".into());
    }
    let mut channels = import
        .channels
        .into_iter()
        .filter(|notes| !notes.is_empty());
    let mut tracks = std::array::from_fn(|_| channels.next().unwrap_or_default());
    if channels.next().is_some() {
        return Err("MIDI has more than three tonal channels.".into());
    }
    if tracks.iter().all(Vec::is_empty) {
        return Err("MIDI contains no tonal notes.".into());
    }
    for track in &mut tracks {
        track.sort_by_key(|note| note.start);
    }
    validate(&tracks)?;
    Ok((tracks, bpm))
}

#[cfg(test)]
#[path = "midi/tests.rs"]
mod tests;

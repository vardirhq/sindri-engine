//! 16-bit mono PCM WAV, the format every Sindri audio backend decodes.
use super::RATE;
use std::error::Error;

/// Encodes normalized samples as a 44.1 kHz, 16-bit RIFF/WAVE file; stereo
/// samples are interleaved, left first.
// Samples are clamped to the signed 16-bit range before the cast.
#[allow(clippy::cast_possible_truncation)]
pub(super) fn encode(samples: &[f32], channels: u16) -> Result<Vec<u8>, Box<dyn Error>> {
    let bytes = u32::try_from(samples.len())?
        .checked_mul(2)
        .ok_or("WAV too long")?;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + bytes).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16_u32.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2 * u32::from(channels)).to_le_bytes());
    out.extend_from_slice(&(2 * channels).to_le_bytes());
    out.extend_from_slice(&16_u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&bytes.to_le_bytes());
    for sample in samples {
        let pcm = (sample.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        out.extend_from_slice(&pcm.to_le_bytes());
    }
    Ok(out)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn write(
    path: &std::path::Path,
    samples: &[f32],
    channels: u16,
) -> Result<(), Box<dyn Error>> {
    std::fs::write(path, encode(samples, channels)?)?;
    Ok(())
}

/// Reads back the PCM payload of a file this module wrote.
#[cfg(test)]
pub(super) fn decode(bytes: &[u8]) -> Option<Vec<i16>> {
    let data = bytes.get(44..)?;
    if bytes.get(..4)? != b"RIFF" || bytes.get(36..40)? != b"data" {
        return None;
    }
    Some(
        data.chunks_exact(2)
            .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
            .collect(),
    )
}

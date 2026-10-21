//! Ogg Vorbis, the format a song is delivered in.
//!
//! A WAV is what a song sounds like, uncompressed: an 80-second stereo loop is
//! fifteen megabytes, which is a download a web game cannot ask for. Vorbis at
//! a high quality setting is about a tenth of that and indistinguishable for
//! this music. Every Sindri host decodes it -- natively through Rodio, in a
//! browser through Web Audio, which Safari has done since 18.4.
//!
//! The stream serial is fixed rather than random, so one song always encodes
//! to the same bytes and a delivered clip can be checked against its source.
use super::RATE;
use std::error::Error;
use std::num::{NonZeroU8, NonZeroU32};
use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoderBuilder};

/// Vorbis's own quality scale, from -0.2 to 1: 0.6 is transparent for
/// synthesized music and still about a tenth of the PCM.
const QUALITY: f32 = 0.6;

/// The same serial for every stream, so encoding is deterministic.
const SERIAL: i32 = 0x5143_4157;

/// Encodes normalized samples, interleaved when stereo, as Ogg Vorbis.
pub(super) fn encode(samples: &[f32], channels: u8) -> Result<Vec<u8>, Box<dyn Error>> {
    let count = NonZeroU8::new(channels).ok_or("a sound has at least one channel")?;
    let rate = NonZeroU32::new(RATE).ok_or("a sample rate is never zero")?;
    let mut encoder = VorbisEncoderBuilder::new_with_serial(rate, count, Vec::new(), SERIAL)
        .bitrate_management_strategy(VorbisBitrateManagementStrategy::QualityVbr {
            target_quality: QUALITY,
        })
        .build()?;
    let width = usize::from(channels);
    // Planar blocks of a second at a time, as the encoder takes them.
    for chunk in samples.chunks(RATE as usize * width) {
        let planes: Vec<Vec<f32>> = (0..width)
            .map(|channel| chunk.iter().skip(channel).step_by(width).copied().collect())
            .collect();
        encoder.encode_audio_block(&planes)?;
    }
    Ok(encoder.finish()?)
}

#[cfg(test)]
mod tests {
    use super::encode;

    /// A song is an Ogg stream, far smaller than its PCM, and the same every
    /// time it is encoded.
    #[test]
    fn a_song_encodes_to_a_small_deterministic_ogg_stream() {
        let seconds = 4.0;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let frames = (f64::from(super::RATE) * seconds) as usize;
        let samples: Vec<f32> = (0..frames * 2)
            .map(|i| {
                #[allow(clippy::cast_precision_loss)]
                let t = (i / 2) as f32 / super::RATE as f32;
                (t * 440.0 * std::f32::consts::TAU).sin() * 0.4
            })
            .collect();
        let first = encode(&samples, 2).expect("encodes");
        assert_eq!(&first[..4], b"OggS", "an Ogg stream");
        assert!(
            first.len() * 8 < samples.len() * 2,
            "{} bytes is not much smaller than {} of 16-bit PCM",
            first.len(),
            samples.len() * 2
        );
        assert_eq!(
            first,
            encode(&samples, 2).expect("encodes again"),
            "deterministic"
        );
    }
}

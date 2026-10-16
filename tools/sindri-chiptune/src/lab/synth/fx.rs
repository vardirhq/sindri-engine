//! The room and the master bus: reverb, echo, chorus, compression and limiting.
use super::dsp::{OnePole, RATE, db};

/// A delay line with fractional reads.
#[derive(Clone, Debug)]
struct Line {
    buffer: Vec<f64>,
    write: usize,
}

impl Line {
    fn new(samples: usize) -> Self {
        Self {
            buffer: vec![0.0; samples.max(2)],
            write: 0,
        }
    }

    fn push(&mut self, value: f64) {
        self.buffer[self.write] = value;
        self.write = (self.write + 1) % self.buffer.len();
    }

    /// The value `delay` samples ago, interpolated between two samples.
    // Delays are bounded by the buffer, far below where f64 loses integers.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn read(&self, delay: f64) -> f64 {
        let len = self.buffer.len();
        let delay = delay.clamp(1.0, (len - 1) as f64);
        let whole = delay.floor();
        let frac = delay - whole;
        let a = (self.write + len - whole as usize) % len;
        let b = (a + len - 1) % len;
        self.buffer[a] * (1.0 - frac) + self.buffer[b] * frac
    }

    fn tail(&self) -> f64 {
        self.buffer[self.write]
    }
}

/// A damped feedback comb, Freeverb's building block.
#[derive(Clone, Debug)]
struct Comb {
    line: Line,
    store: f64,
}

impl Comb {
    fn tick(&mut self, input: f64, feedback: f64, damp: f64) -> f64 {
        let out = self.line.tail();
        self.store = out * (1.0 - damp) + self.store * damp;
        self.line.push(input + self.store * feedback);
        out
    }
}

#[derive(Clone, Debug)]
struct Allpass {
    line: Line,
}

impl Allpass {
    fn tick(&mut self, input: f64) -> f64 {
        let delayed = self.line.tail();
        self.line.push(input + delayed * 0.5);
        delayed - input
    }
}

/// Jezar's Freeverb: eight combs and four all-passes per side, the right side
/// detuned by a few samples so the room is wide.
#[derive(Clone, Debug)]
pub(super) struct Reverb {
    combs: [Vec<Comb>; 2],
    passes: [Vec<Allpass>; 2],
    pre: Line,
    tone: [OnePole; 2],
}

impl Reverb {
    pub(super) fn new() -> Self {
        const COMBS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
        const PASSES: [usize; 4] = [556, 441, 341, 225];
        const SPREAD: usize = 23;
        let side = |offset: usize| {
            (
                COMBS
                    .iter()
                    .map(|n| Comb {
                        line: Line::new(n + offset),
                        store: 0.0,
                    })
                    .collect(),
                PASSES
                    .iter()
                    .map(|n| Allpass {
                        line: Line::new(n + offset),
                    })
                    .collect(),
            )
        };
        let (left_combs, left_passes) = side(0);
        let (right_combs, right_passes) = side(SPREAD);
        Self {
            combs: [left_combs, right_combs],
            passes: [left_passes, right_passes],
            pre: Line::new(4_096),
            tone: [OnePole::default(); 2],
        }
    }

    /// `size` 0..1 is the room's length; returns the wet stereo signal.
    // Each side owns parallel comb and all-pass banks.
    #[allow(clippy::needless_range_loop)]
    pub(super) fn tick(&mut self, input: [f64; 2], size: f64) -> [f64; 2] {
        // A short pre-delay keeps the dry attack clear of the room.
        let mono = (input[0] + input[1]) * 0.5;
        self.pre.push(mono);
        let delayed = self.pre.read(0.018 * RATE) * 0.015;
        let feedback = 0.7 + 0.28 * size.clamp(0.0, 1.0);
        let mut out = [0.0; 2];
        for side in 0..2 {
            let mut sum = 0.0;
            for comb in &mut self.combs[side] {
                sum += comb.tick(delayed, feedback, 0.35);
            }
            for pass in &mut self.passes[side] {
                sum = pass.tick(sum);
            }
            // Darken the tail, as a real room does.
            out[side] = self.tone[side].low(sum, 6_500.0);
        }
        out
    }
}

/// A tempo-synced ping-pong echo, its repeats darker each time round.
#[derive(Clone, Debug)]
pub(super) struct Echo {
    lines: [Line; 2],
    tone: [OnePole; 2],
    low_cut: [OnePole; 2],
}

impl Echo {
    pub(super) fn new() -> Self {
        let seconds = 2.0;
        // Two seconds covers a dotted eighth down to 45 BPM.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let size = (seconds * RATE) as usize;
        Self {
            lines: [Line::new(size), Line::new(size)],
            tone: [OnePole::default(); 2],
            low_cut: [OnePole::default(); 2],
        }
    }

    pub(super) fn tick(&mut self, input: [f64; 2], delay_samples: f64, feedback: f64) -> [f64; 2] {
        let left = self.lines[0].read(delay_samples);
        let right = self.lines[1].read(delay_samples);
        // Left feeds right and right feeds left: the repeats bounce.
        let into_left = (input[0] + input[1]) * 0.5 + right * feedback;
        let into_right = left * feedback;
        let shaped_left = self.low_cut[0].high(self.tone[0].low(into_left, 4_500.0), 180.0);
        let shaped_right = self.low_cut[1].high(self.tone[1].low(into_right, 4_500.0), 180.0);
        self.lines[0].push(shaped_left);
        self.lines[1].push(shaped_right);
        [left, right]
    }
}

/// A two-voice chorus that spreads a mono part across the stereo field.
#[derive(Clone, Debug)]
pub(super) struct Chorus {
    line: Line,
    phase: f64,
}

impl Chorus {
    pub(super) fn new() -> Self {
        Self {
            line: Line::new(2_048),
            phase: 0.0,
        }
    }

    pub(super) fn tick(&mut self, input: f64) -> [f64; 2] {
        self.line.push(input);
        self.phase = (self.phase + 0.6 / RATE).fract();
        let sweep = (std::f64::consts::TAU * self.phase).sin();
        let base = 0.012 * RATE;
        let depth = 0.003 * RATE;
        let left = self.line.read(base + depth * sweep);
        let right = self.line.read(base - depth * sweep);
        [input * 0.6 + left * 0.5, input * 0.6 + right * 0.5]
    }
}

/// A feed-forward bus compressor that glues the parts into one mix.
#[derive(Clone, Copy, Debug)]
pub(super) struct Compressor {
    envelope: f64,
}

impl Compressor {
    pub(super) fn new() -> Self {
        Self { envelope: 0.0 }
    }

    pub(super) fn tick(&mut self, input: [f64; 2], threshold_db: f64, ratio: f64) -> [f64; 2] {
        let level = input[0].abs().max(input[1].abs());
        let coefficient = if level > self.envelope {
            1.0 - (-1.0 / (0.010 * RATE)).exp()
        } else {
            1.0 - (-1.0 / (0.180 * RATE)).exp()
        };
        self.envelope += (level - self.envelope) * coefficient;
        let over = 20.0 * self.envelope.max(1e-6).log10() - threshold_db;
        let reduction = if over > 0.0 {
            over * (1.0 - 1.0 / ratio)
        } else {
            0.0
        };
        let gain = db(-reduction);
        [input[0] * gain, input[1] * gain]
    }
}

/// A brick wall with a short look-ahead, then a soft knee for anything left:
/// the mix is loud without ever clipping.
#[derive(Clone, Debug)]
pub(super) struct Limiter {
    lines: [Line; 2],
    gain: f64,
    hold: Vec<f64>,
    write: usize,
}

const LOOKAHEAD: usize = 64;

impl Limiter {
    pub(super) fn new() -> Self {
        Self {
            lines: [Line::new(LOOKAHEAD + 2), Line::new(LOOKAHEAD + 2)],
            gain: 1.0,
            hold: vec![0.0; LOOKAHEAD],
            write: 0,
        }
    }

    // LOOKAHEAD is a small constant.
    #[allow(clippy::cast_precision_loss)]
    pub(super) fn tick(&mut self, input: [f64; 2], ceiling: f64) -> [f64; 2] {
        let peak = input[0].abs().max(input[1].abs());
        self.hold[self.write] = peak;
        self.write = (self.write + 1) % LOOKAHEAD;
        let ahead = self.hold.iter().fold(0.0_f64, |a, b| a.max(*b));
        let target = if ahead > ceiling {
            ceiling / ahead
        } else {
            1.0
        };
        if target < self.gain {
            self.gain = target;
        } else {
            self.gain += (target - self.gain) * (1.0 - (-1.0 / (0.120 * RATE)).exp());
        }
        let mut out = [0.0; 2];
        for (side, value) in out.iter_mut().enumerate() {
            self.lines[side].push(input[side]);
            let delayed = self.lines[side].read(LOOKAHEAD as f64) * self.gain;
            *value = soft_clip(delayed / ceiling) * ceiling;
        }
        out
    }
}

/// Linear to 0.8, then a smooth knee that never exceeds 1.
fn soft_clip(x: f64) -> f64 {
    let a = x.abs();
    if a <= 0.8 {
        x
    } else {
        let over = (a - 0.8) / 0.2;
        x.signum() * (0.8 + 0.2 * over.tanh())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_limiter_never_exceeds_its_ceiling() {
        let mut limiter = Limiter::new();
        let mut loudest = 0.0_f64;
        for i in 0..44_100 {
            let x = (f64::from(i) * 0.05).sin() * 3.0;
            let out = limiter.tick([x, -x], 0.9);
            loudest = loudest.max(out[0].abs()).max(out[1].abs());
        }
        assert!(loudest <= 0.9 + 1e-9, "{loudest}");
    }

    #[test]
    fn the_reverb_rings_on_after_its_input_stops() {
        let mut reverb = Reverb::new();
        reverb.tick([1.0, 1.0], 0.8);
        let mut tail = 0.0_f64;
        for i in 0..44_100 {
            let out = reverb.tick([0.0, 0.0], 0.8);
            if i > 22_050 {
                tail = tail.max(out[0].abs());
            }
        }
        assert!(tail > 1e-5, "{tail}");
    }

    #[test]
    fn the_echo_bounces_between_sides() {
        let mut echo = Echo::new();
        echo.tick([1.0, 1.0], 100.0, 0.5);
        let mut first = [0usize; 2];
        for i in 1..400 {
            let out = echo.tick([0.0, 0.0], 100.0, 0.5);
            for side in 0..2 {
                if first[side] == 0 && out[side].abs() > 1e-3 {
                    first[side] = i;
                }
            }
        }
        assert!(first[0] > 0 && first[1] > first[0], "{first:?}");
    }
}

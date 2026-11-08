//! Sound effects, generated in code: meows, purr, crunch and lap.
//!
//! The game ships without audio files. Everything works on mono `f32` samples
//! at [`SAMPLE_RATE`] and is turned into an in-memory WAV file with
//! [`wav_bytes`] for Macroquad to play.

use std::f32::consts::TAU;

pub const SAMPLE_RATE: u32 = 44_100;

const SAMPLE_RATE_F32: f32 = SAMPLE_RATE as f32;

/// The meows the cat can make, as `(pitch in Hz, length in seconds)`: a
/// medium one, a deeper longer one and a higher shorter one.
pub const MEOWS: [(f32, f32); 3] = [(830.0, 0.46), (720.0, 0.60), (900.0, 0.38)];

/// The purr is a one-second loop with this many pulses in it.
const PURR_PULSES_PER_LOOP: f32 = 25.0;

fn samples_for(seconds: f32) -> usize {
    (seconds * SAMPLE_RATE_F32) as usize
}

/// Tiny deterministic white-noise source (xorshift32), so the generated
/// sounds are identical on every run.
struct Noise(u32);

impl Noise {
    fn new(seed: u32) -> Self {
        // The state must never be zero.
        Self(seed.wrapping_mul(2_654_435_761) | 1)
    }

    /// Next value in `-1.0..=1.0`.
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f32 / u32::MAX as f32 * 2.0 - 1.0
    }
}

/// A two-pole band-pass filter. Feeding noise through it keeps just the part
/// of the sound around `freq`, which is how the crunches get their crack.
struct Resonator {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Resonator {
    const fn new() -> Self {
        Self {
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    /// Filters one sample, centred on `freq` Hz; higher `q` = narrower band.
    fn process(&mut self, x: f32, freq: f32, q: f32) -> f32 {
        let w0 = TAU * freq / SAMPLE_RATE_F32;
        let alpha = w0.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        let gain = alpha / a0;
        let a1 = -2.0 * w0.cos() / a0;
        let a2 = (1.0 - alpha) / a0;

        let y = gain * (x - self.x2) - a1 * self.y1 - a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// Scales the samples so the loudest one reaches `target_peak`.
fn normalize(samples: &mut [f32], target_peak: f32) {
    let peak = samples.iter().fold(0.0_f32, |max, s| max.max(s.abs()));
    if peak > 0.0 {
        let gain = target_peak / peak;
        for sample in samples {
            *sample *= gain;
        }
    }
}

/// Linear interpolation through `(position, value)` keyframes, sorted by
/// position. Outside the first and last keyframe the value stays put.
fn curve(keys: &[(f32, f32)], at: f32) -> f32 {
    let Some(&(first_at, first_value)) = keys.first() else {
        return 0.0;
    };
    if at <= first_at {
        return first_value;
    }
    for pair in keys.windows(2) {
        let ((from_at, from), (to_at, to)) = (pair[0], pair[1]);
        if at <= to_at {
            return from + (to - from) * (at - from_at) / (to_at - from_at);
        }
    }
    keys.last().map_or(0.0, |&(_, value)| value)
}

// The shape of a meow, as measured from a real recording. Positions are the
// fraction (0.0 to 1.0) of the way through the meow.

/// Pitch relative to the meow's top note: a quick rise, then a slow slide down.
const MEOW_PITCH: [(f32, f32); 6] = [
    (0.0, 0.94),
    (0.09, 1.0),
    (0.3, 0.947),
    (0.55, 0.93),
    (0.8, 0.855),
    (1.0, 0.80),
];
/// Loudness: a sharp attack, then a long fade.
const MEOW_LOUDNESS: [(f32, f32); 14] = [
    (0.0, 0.0),
    (0.043, 0.9),
    (0.109, 1.0),
    (0.196, 0.93),
    (0.283, 0.82),
    (0.37, 0.67),
    (0.457, 0.43),
    (0.543, 0.34),
    (0.63, 0.34),
    (0.717, 0.28),
    (0.804, 0.18),
    (0.891, 0.08),
    (0.978, 0.02),
    (1.0, 0.0),
];
/// The throat's low resonance (Hz): it stays put, then sinks at the end.
const MEOW_FORMANT_1: [(f32, f32); 3] = [(0.0, 770.0), (0.5, 770.0), (1.0, 640.0)];
/// The high resonance (Hz), which glides down...
const MEOW_FORMANT_2: [(f32, f32); 4] =
    [(0.0, 2350.0), (0.5, 2400.0), (0.75, 1700.0), (1.0, 1300.0)];
/// ...and fades out in the last third, which makes the meow go "dull".
const MEOW_FORMANT_2_LEVEL: [(f32, f32); 4] = [(0.0, 1.0), (0.5, 0.9), (0.75, 0.1), (1.0, 0.0)];

/// How strong the high resonance is compared with the low one. The low
/// resonance sits right on the voice's pitch and would otherwise swamp
/// everything; a real meow has most of its energy in the nasal 1 to 4 kHz.
const MEOW_FORMANT_2_GAIN: f32 = 3.5;
const MEOW_FORMANT_3_GAIN: f32 = 0.6;

/// A cat's meow: a buzzy voice at about `top_hz` run through two vowel-like
/// resonances, shaped to follow a real meow's pitch, loudness and tone.
/// `seed` only changes the tiny random wobble in the voice and breath.
pub fn meow(top_hz: f32, seconds: f32, seed: u32) -> Vec<f32> {
    let n = samples_for(seconds);
    let mut noise = Noise::new(seed);
    let (mut formant_1, mut formant_2, mut formant_3) =
        (Resonator::new(), Resonator::new(), Resonator::new());
    let mut phase = 0.0_f32;

    let mut out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;

            let hz = top_hz * curve(&MEOW_PITCH, t) * (1.0 + 0.003 * noise.next());
            phase = (phase + hz / SAMPLE_RATE_F32).fract();
            let source = (2.0 * phase - 1.0) + 0.03 * noise.next();

            let vowel = formant_1.process(source, curve(&MEOW_FORMANT_1, t), 7.0)
                + MEOW_FORMANT_2_GAIN
                    * curve(&MEOW_FORMANT_2_LEVEL, t)
                    * formant_2.process(source, curve(&MEOW_FORMANT_2, t), 9.0)
                + MEOW_FORMANT_3_GAIN * formant_3.process(source, 3300.0, 10.0);
            vowel * curve(&MEOW_LOUDNESS, t)
        })
        .collect();
    normalize(&mut out, 0.85);
    out
}

/// How hard the purr "pushes" at time `t` (`0.0..=1.0` through the loop).
///
/// The rate wobbles around 25 Hz like a real purr, but always completes
/// exactly 25 pulses per loop, and it is silent at both ends so the loop has
/// no click at the seam.
fn purr_pulse(t: f32) -> f32 {
    let wobble = 1.5 / TAU * (1.0 - (TAU * t).cos());
    let phase = PURR_PULSES_PER_LOOP * t + wobble;
    (0.5 - 0.5 * (TAU * phase).cos()).powf(1.5)
}

/// One second of purring, made to be played in a loop: a low rumble of
/// filtered noise, chopped into pulses about 25 times a second.
pub fn purr() -> Vec<f32> {
    let n = samples_for(1.0);
    let mut noise = Noise::new(7);
    let (mut low, mut mid, mut high) = (Resonator::new(), Resonator::new(), Resonator::new());

    let mut out: Vec<f32> = (0..n)
        .map(|i| {
            let grit = noise.next();
            let rumble = low.process(grit, 110.0, 1.0)
                + 0.8 * mid.process(grit, 420.0, 1.2)
                + 0.35 * high.process(grit, 1100.0, 1.0);
            rumble * purr_pulse(i as f32 / n as f32)
        })
        .collect();
    normalize(&mut out, 0.8);
    out
}

fn decay_after(t: f32, onset: f32, rate: f32) -> f32 {
    if t < onset {
        0.0
    } else {
        (-(t - onset) * rate).exp()
    }
}

/// Mixes one bite into `buf` at `at` seconds: a crack of band-passed noise
/// (three quick fractures) on top of a dull thud.
fn add_crunch(buf: &mut [f32], at: f32, strength: f32, bright_hz: f32, noise: &mut Noise) {
    let mut crack = Resonator::new();
    let bite = buf.iter_mut().skip(samples_for(at)).take(samples_for(0.14));
    for (k, slot) in bite.enumerate() {
        let t = k as f32 / SAMPLE_RATE_F32;
        let envelope = (-t * 70.0).exp()
            + 0.7 * decay_after(t, 0.022, 90.0)
            + 0.4 * decay_after(t, 0.05, 110.0);
        let thud = (TAU * 140.0 * t).sin() * (-t * 45.0).exp() * 0.5;
        *slot += strength * (crack.process(noise.next(), bright_hz, 1.5) * envelope + thud);
    }
}

/// Three crunchy bites in half a second. Different seeds give different
/// noise, so alternating them avoids sounding like a loop.
pub fn crunch(seed: u32) -> Vec<f32> {
    let mut noise = Noise::new(seed);
    let mut out = vec![0.0; samples_for(0.52)];
    for (at, strength, bright_hz) in [(0.0, 1.0, 2400.0), (0.18, 0.8, 3000.0), (0.36, 0.9, 2000.0)]
    {
        add_crunch(&mut out, at, strength, bright_hz, &mut noise);
    }
    normalize(&mut out, 0.8);
    out
}

/// Mixes one lap into `buf` at `at` seconds: a bubbly blip that glides up in
/// pitch, with a little splash of noise.
fn add_lap(buf: &mut [f32], at: f32, strength: f32, noise: &mut Noise) {
    let mut splash = Resonator::new();
    let mut phase = 0.0_f32;
    let lap = buf.iter_mut().skip(samples_for(at)).take(samples_for(0.14));
    for (k, slot) in lap.enumerate() {
        let t = k as f32 / SAMPLE_RATE_F32;
        let hz = 320.0 + 600.0 * (t / 0.08).min(1.0);
        phase = (phase + hz / SAMPLE_RATE_F32).fract();
        let envelope = (t * 250.0).min(1.0) * (-t * 30.0).exp();
        let blip = (TAU * phase).sin() * 0.7;
        let wet = splash.process(noise.next(), 1300.0, 2.0) * 0.35;
        *slot += strength * envelope * (blip + wet);
    }
}

/// Three soft laps in half a second, for drinking water or milk.
pub fn lap(seed: u32) -> Vec<f32> {
    let mut noise = Noise::new(seed);
    let mut out = vec![0.0; samples_for(0.5)];
    for (at, strength) in [(0.0, 1.0), (0.17, 0.8), (0.34, 0.9)] {
        add_lap(&mut out, at, strength, &mut noise);
    }
    normalize(&mut out, 0.7);
    out
}

/// Encodes `samples` (clamped to `-1.0..=1.0`) as a mono 16-bit PCM WAV file.
pub fn wav_bytes(samples: &[f32]) -> Vec<u8> {
    let data_len = samples.len() * 2;
    let mut bytes = Vec::with_capacity(44 + data_len);

    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes()); // size of the fmt chunk
    bytes.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1_u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // bytes per second
    bytes.extend_from_slice(&2_u16.to_le_bytes()); // bytes per frame
    bytes.extend_from_slice(&16_u16.to_le_bytes()); // bits per sample
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data_len as u32).to_le_bytes());
    for sample in samples {
        let value = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peak(samples: &[f32]) -> f32 {
        samples.iter().fold(0.0, |max, s| f32::max(max, s.abs()))
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    }

    /// RMS of the part of `samples` between `from` and `to` seconds.
    fn rms_between(samples: &[f32], from: f32, to: f32) -> f32 {
        rms(&samples[samples_for(from)..samples_for(to)])
    }

    /// Estimates the pitch (Hz) of the part of `samples` between `from` and
    /// `to` seconds, as the delay at which the sound best matches itself.
    fn pitch_between(samples: &[f32], from: f32, to: f32) -> f32 {
        let part = &samples[samples_for(from)..samples_for(to)];
        let correlation =
            |lag: usize| -> f32 { part.iter().zip(&part[lag..]).map(|(a, b)| a * b).sum() };
        let lags = SAMPLE_RATE as usize / 1800..SAMPLE_RATE as usize / 150;
        let best = lags
            .max_by(|&a, &b| correlation(a).total_cmp(&correlation(b)))
            .expect("some lags");
        SAMPLE_RATE_F32 / best as f32
    }

    fn assert_near(actual: f32, expected: f32, tolerance: f32, what: &str) {
        let error = (actual - expected).abs() / expected;
        assert!(
            error <= tolerance,
            "{what}: {actual:.0} is not within {:.0}% of {expected:.0}",
            tolerance * 100.0
        );
    }

    fn meows() -> Vec<Vec<f32>> {
        MEOWS
            .iter()
            .zip(1..)
            .map(|(&(hz, seconds), seed)| meow(hz, seconds, seed))
            .collect()
    }

    fn all_sounds() -> Vec<(&'static str, Vec<f32>)> {
        let mut sounds = vec![
            ("purr", purr()),
            ("crunch 1", crunch(1)),
            ("crunch 2", crunch(2)),
            ("lap", lap(3)),
        ];
        let names = ["meow 1", "meow 2", "meow 3"];
        sounds.extend(names.into_iter().zip(meows()));
        sounds
    }

    #[test]
    fn every_sound_is_audible_finite_and_does_not_clip() {
        for (name, samples) in all_sounds() {
            assert!(samples.iter().all(|s| s.is_finite()), "{name} has NaN/inf");
            let peak = peak(&samples);
            assert!((0.5..=1.0).contains(&peak), "{name}: peak {peak}");
            assert!(rms(&samples) > 0.02, "{name} is nearly silent");
        }
    }

    #[test]
    fn sounds_are_short_enough_to_trigger_often() {
        assert_eq!(purr().len(), SAMPLE_RATE as usize);
        for samples in [crunch(1), lap(1)] {
            let seconds = samples.len() as f32 / SAMPLE_RATE_F32;
            assert!((0.4..=0.7).contains(&seconds), "{seconds} s");
        }
    }

    #[test]
    fn generated_sounds_are_deterministic() {
        assert_eq!(crunch(5), crunch(5));
        assert_ne!(crunch(1), crunch(2), "different seeds should differ");
        assert_eq!(purr(), purr());
    }

    #[test]
    fn purr_pulses_25_times_per_loop_and_is_silent_at_the_seam() {
        assert!(purr_pulse(0.0) < 1e-3);
        assert!(purr_pulse(1.0) < 1e-3);

        // Count the peaks of the pulse envelope.
        let steps = 10_000;
        let envelope: Vec<f32> = (0..=steps)
            .map(|i| purr_pulse(i as f32 / steps as f32))
            .collect();
        let peaks = envelope
            .windows(3)
            .filter(|w| w[1] > w[0] && w[1] >= w[2] && w[1] > 0.5)
            .count();
        assert_eq!(peaks, PURR_PULSES_PER_LOOP as usize);

        let samples = purr();
        assert!(samples[0].abs() < 0.01, "start {}", samples[0]);
        assert!(samples[samples.len() - 1].abs() < 0.01, "end");
    }

    #[test]
    fn each_bite_of_a_crunch_dies_away_before_the_next() {
        let samples = crunch(1);
        let attack = rms_between(&samples, 0.0, 0.03);
        let gap = rms_between(&samples, 0.13, 0.17);
        assert!(gap < attack * 0.1, "gap {gap} vs bite {attack}");
        // ...and the second bite really does start again.
        assert!(rms_between(&samples, 0.18, 0.21) > gap * 5.0);
    }

    #[test]
    fn each_lap_dies_away_before_the_next() {
        let samples = lap(1);
        let gap = rms_between(&samples, 0.14, 0.17);
        assert!(gap < rms_between(&samples, 0.0, 0.03) * 0.1);
        assert!(rms_between(&samples, 0.17, 0.2) > gap * 5.0);
    }

    #[test]
    fn wav_bytes_form_a_valid_mono_16_bit_file() {
        let samples = crunch(1);
        let bytes = wav_bytes(&samples);

        let mut reader = hound::WavReader::new(std::io::Cursor::new(bytes)).expect("valid WAV");
        let spec = reader.spec();
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.sample_rate, SAMPLE_RATE);
        assert_eq!(spec.bits_per_sample, 16);
        assert_eq!(spec.sample_format, hound::SampleFormat::Int);

        let decoded: Vec<i16> = reader.samples::<i16>().map(Result::unwrap).collect();
        assert_eq!(decoded.len(), samples.len());
        for (d, s) in decoded.iter().zip(&samples) {
            assert!((f32::from(*d) / 32767.0 - s).abs() < 1e-3);
        }
    }

    #[test]
    fn meow_follows_a_real_cats_pitch_contour() {
        // Measured from a real meow: it jumps up to its top note almost at
        // once, then slides down by about 15% before it ends.
        let (top_hz, seconds) = MEOWS[0];
        let samples = meow(top_hz, seconds, 1);

        let start = pitch_between(&samples, 0.01, 0.05);
        let top = pitch_between(&samples, 0.03, 0.07);
        let middle = pitch_between(&samples, 0.17, 0.21);
        let end = pitch_between(&samples, 0.30, 0.34);

        assert_near(top, top_hz, 0.06, "top note");
        assert_near(start, top_hz * 0.96, 0.06, "start");
        assert_near(middle, top_hz * 0.94, 0.06, "middle");
        assert_near(end, top_hz * 0.89, 0.07, "end");
        assert!(end < top * 0.95, "pitch should fall: {top:.0} -> {end:.0}");
    }

    #[test]
    fn a_meow_starts_fast_and_fades_out() {
        for samples in meows() {
            let seconds = samples.len() as f32 / SAMPLE_RATE_F32;
            let loud = rms_between(&samples, 0.05 * seconds, 0.2 * seconds);
            let quiet = rms_between(&samples, 0.75 * seconds, seconds - 0.001);
            assert!(loud > quiet * 4.0, "loud {loud} vs end {quiet}");

            // Starts and ends from silence, so there is no click.
            assert!(samples[0].abs() < 0.01 && samples[samples.len() - 1].abs() < 0.01);
            // The attack is quick: already loud 25 ms in.
            assert!(rms_between(&samples, 0.02, 0.03) > loud * 0.5);
        }
    }

    #[test]
    fn the_three_meows_differ_in_pitch_and_length() {
        let all = meows();
        let (medium, deep, high) = (&all[0], &all[1], &all[2]);

        assert!(deep.len() > medium.len() && medium.len() > high.len());
        let [medium_hz, deep_hz, high_hz] =
            [medium, deep, high].map(|m| pitch_between(m, 0.03, 0.07));
        assert!(deep_hz < medium_hz && medium_hz < high_hz);
        assert_near(deep_hz, MEOWS[1].0, 0.06, "deep meow");
        assert_near(high_hz, MEOWS[2].0, 0.06, "high meow");
    }

    #[test]
    fn curve_interpolates_and_holds_at_the_ends() {
        let keys = [(0.0, 10.0), (1.0, 20.0), (3.0, 0.0)];
        assert!((curve(&keys, -5.0) - 10.0).abs() < 1e-6);
        assert!((curve(&keys, 0.5) - 15.0).abs() < 1e-6);
        assert!((curve(&keys, 1.0) - 20.0).abs() < 1e-6);
        assert!((curve(&keys, 2.0) - 10.0).abs() < 1e-6);
        assert!((curve(&keys, 9.0) - 0.0).abs() < 1e-6);
        assert!(curve(&[], 1.0).abs() < 1e-6);
    }

    /// Writes every sound to `$CAT_SOUND_DUMP_DIR` so they can be played back
    /// and inspected outside the game:
    /// `CAT_SOUND_DUMP_DIR=/some/empty/dir cargo test dump_sounds -- --ignored`
    #[test]
    #[ignore = "writes files; run by hand to listen to the sounds"]
    fn dump_sounds_for_listening() {
        let Some(dir) = std::env::var_os("CAT_SOUND_DUMP_DIR") else {
            panic!("set CAT_SOUND_DUMP_DIR to the folder to write the sounds to");
        };
        for (name, samples) in all_sounds() {
            let file = std::path::Path::new(&dir).join(format!("{}.wav", name.replace(' ', "-")));
            std::fs::write(&file, wav_bytes(&samples)).expect("write sound");
        }
    }
}

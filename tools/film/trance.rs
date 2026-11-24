// The trailer's music: acid trance at 140 BPM in A minor, written to a 16-bit stereo WAV, one Rust
// file and no dependency, like acid.rs. Everything is synthesized here:
//
// - a kick on every beat, and every other part ducking under it (the "pump");
// - a rolling bass, three sixteenths after every kick, on the root of the chord;
// - the acid line: a sawtooth through a four-pole resonant low-pass after the Roland TB-303, its
//   cutoff opened by an envelope on every note, wider on accents, sliding between some notes, and
//   opening further over each section;
// - pads: three-note chords of seven detuned sawtooths each, low-passed, slow to come in;
// - an arpeggio of the chord's notes in sixteenths, plucked, with a dotted-eighth echo;
// - hats (closed sixteenths, open on the off-beats), a clap on two and four;
// - a white-noise riser and a snare roll building into each drop, and a crash on the drop.
//
// 8 bars and the last chord ringing, 15 s, for a trailer of 15 s: the build (2 bars: pads, the
// acid line closed, the riser and the snare roll), the drop (5 bars: everything, the acid line
// opening, the arpeggio), and the end (1 bar and a little: A minor ringing out under the title).
//
//     rustc -O tools/film/trance.rs -o target/trailer/trance.exe && target/trailer/trance.exe target/trailer/trance.wav

use std::f32::consts::{PI, TAU};
use std::fs::File;
use std::io::{BufWriter, Write};

const RATE: f32 = 44_100.0;
const BPM: f32 = 140.0;
const BARS: usize = 8;

/// Where each section begins, in bars.
const DROP1: usize = 2;
const END: usize = 7;

struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn hz(semitones_from_a: f32, octave_a: f32) -> f32 {
    octave_a * 2f32.powf(semitones_from_a / 12.0)
}

/// The chords, one a bar, two bars each: Am, F, C, G, as semitones from A.
fn chord(bar: usize) -> [i32; 3] {
    if bar >= END {
        return [0, 3, 7];
    }
    match (bar / 2) % 4 {
        0 => [0, 3, 7],    // A C E
        1 => [-4, 0, 3],   // F A C
        2 => [3, 7, 10],   // C E G
        _ => [-2, 2, 5],   // G B D
    }
}

/// The chord's root, as semitones from A.
fn root(bar: usize) -> i32 {
    chord(bar)[0]
}

// One step of the acid line: semitones from A2 (None is a rest), accent, slide into the next.
type Step = (Option<i32>, bool, bool);

fn acid_pattern(second: bool) -> [Step; 16] {
    let mut p = [
        (Some(0), true, false),
        (Some(12), false, true),
        (Some(0), false, false),
        (Some(0), false, false),
        (Some(15), true, false),
        (Some(0), false, false),
        (Some(12), false, true),
        (Some(10), false, false),
        (Some(0), true, false),
        (None, false, false),
        (Some(7), false, true),
        (Some(12), false, false),
        (Some(0), false, false),
        (Some(3), true, true),
        (Some(5), false, false),
        (Some(12), false, false),
    ];
    if second {
        p[9] = (Some(19), true, false);
        p[15] = (Some(17), true, true);
    }
    p
}

struct Track {
    left: Vec<f32>,
    right: Vec<f32>,
}
impl Track {
    fn add(&mut self, at: usize, l: f32, r: f32) {
        if at < self.left.len() {
            self.left[at] += l;
            self.right[at] += r;
        }
    }
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "trance.wav".into());
    let step_seconds = 60.0 / BPM / 4.0;
    let step_samples = step_seconds * RATE;
    let total_steps = BARS * 16;
    let music_len = (total_steps as f32 * step_samples) as usize;
    let length = music_len + (RATE as usize * 4);
    let mut rng = Rng(0x7a11_cafe);
    let bar_of = |i: usize| ((i as f32 / step_samples) as usize) / 16;
    let kick_on = |bar: usize| (DROP1..END).contains(&bar);

    // The pump: how far every other part ducks under the kick, sample by sample.
    let mut duck = vec![1.0f32; length];
    for step in (0..total_steps).step_by(4) {
        if !kick_on(step / 16) {
            continue;
        }
        let at = (step as f32 * step_samples) as usize;
        for i in 0..(0.42 * step_samples * 4.0) as usize {
            if at + i < length {
                let t = i as f32 / RATE;
                duck[at + i] = duck[at + i].min(1.0 - 0.75 * (-t / 0.09).exp());
            }
        }
    }

    // The drums, the bass and the effects go straight in; the pads, the acid line and the arpeggio
    // go through the pump, and the arpeggio and the pads through the echo too.
    let mut dry = Track { left: vec![0.0; length], right: vec![0.0; length] };
    let mut pumped = Track { left: vec![0.0; length], right: vec![0.0; length] };
    let mut echo_send = Track { left: vec![0.0; length], right: vec![0.0; length] };

    for step in 0..total_steps {
        let bar = step / 16;
        let at = (step as f32 * step_samples) as usize;
        let in_bar = step % 16;

        // Kick.
        if in_bar % 4 == 0 && kick_on(bar) {
            let mut phase = 0.0f32;
            for i in 0..(0.38 * RATE) as usize {
                let t = i as f32 / RATE;
                let pitch = 48.0 + 140.0 * (-t * 32.0).exp();
                phase += pitch / RATE;
                let amp = (-t * 8.0).exp() * (t / 0.0015).min(1.0);
                let click = if t < 0.003 { (1.0 - t / 0.003) * 0.35 } else { 0.0 };
                let s = ((TAU * phase).sin() * amp * 1.1).tanh() + click;
                dry.add(at + i, s, s);
            }
        }

        // The rolling bass: three sixteenths after each kick.
        if in_bar % 4 != 0 && kick_on(bar) {
            let f = hz(root(bar) as f32, 55.0);
            let mut phase = 0.0f32;
            let mut lp = 0.0f32;
            for i in 0..(step_samples * 0.9) as usize {
                let t = i as f32 / RATE;
                phase = (phase + f / RATE).fract();
                let saw = 2.0 * phase - 1.0;
                let cutoff = 300.0 + 1100.0 * (-t * 30.0).exp();
                let g = 1.0 - (-TAU * cutoff / RATE).exp();
                lp += g * (saw - lp);
                let env = (t / 0.002).min(1.0) * (-t * 14.0).exp();
                let s = lp * env * 0.55;
                dry.add(at + i, s, s);
            }
        }

        // Hats: closed sixteenths, open on the off-beat.
        let hats = (DROP1 + 1..END).contains(&bar);
        let offbeat_only = (DROP1..DROP1 + 1).contains(&bar);
        if hats || (offbeat_only && in_bar % 4 == 2) {
            let open = in_bar % 4 == 2;
            let (level, decay) = if open { (0.16, 22.0) } else { (0.05, 95.0) };
            let mut last = 0.0f32;
            let mut last2 = 0.0f32;
            for i in 0..(0.28 * RATE) as usize {
                let t = i as f32 / RATE;
                let n = rng.next();
                let high = n - 2.0 * last + last2;
                last2 = last;
                last = n;
                let s = high * level * (-t * decay).exp();
                dry.add(at + i, s * 0.85, s * 1.1);
            }
        }

        // Clap on two and four.
        let claps = (DROP1 + 1..END).contains(&bar);
        if claps && (in_bar == 4 || in_bar == 12) {
            let mut band = (0.0f32, 0.0f32);
            let tune = 2.0 * (PI * 1300.0 / RATE).sin();
            for i in 0..(0.32 * RATE) as usize {
                let t = i as f32 / RATE;
                let burst = [0.0f32, 0.01, 0.02]
                    .iter()
                    .map(|s| if t >= *s { (-(t - s) * 150.0).exp() } else { 0.0 })
                    .fold(0.0, f32::max)
                    .max((-t * 18.0).exp() * 0.6);
                band.0 += tune * band.1;
                let high = rng.next() - band.0 - 0.7 * band.1;
                band.1 += tune * high;
                let s = band.1 * burst * 0.7;
                dry.add(at + i, s * 1.05, s * 0.95);
            }
        }

        // The snare roll into each drop: eighths, then sixteenths, then thirty-seconds, rising.
        for (from, to) in [(0, DROP1)] {
            if (from..to).contains(&bar) {
                let progress = ((step - from * 16) as f32) / (((to - from) * 16) as f32);
                let per_step: usize = if progress < 0.5 { 1 } else { 2 };
                let every_other = progress < 0.25 && in_bar % 2 == 1;
                if !every_other {
                    for k in 0..per_step {
                        let sub = at + (k as f32 * step_samples / per_step as f32) as usize;
                        let tone = 180.0 + 220.0 * progress;
                        let level = 0.12 + 0.35 * progress * progress;
                        let mut phase = 0.0f32;
                        for i in 0..(0.12 * RATE) as usize {
                            let t = i as f32 / RATE;
                            phase += tone / RATE;
                            let s = (rng.next() * 0.8 + (TAU * phase).sin() * 0.4) * level * (-t * 38.0).exp();
                            dry.add(sub + i, s, s);
                        }
                    }
                }
            }
        }
    }

    // The crash on the drop and on the end: a long wash of noise, bright, and a low boom under it.
    for drop in [DROP1, END] {
        let at = (drop as f32 * 16.0 * step_samples) as usize;
        let mut last = 0.0f32;
        for i in 0..(2.6 * RATE) as usize {
            let t = i as f32 / RATE;
            let n = rng.next();
            let high = n - last;
            last = n;
            let wash = high * 0.22 * (-t * 1.6).exp();
            let boom = (TAU * 42.0 * t).sin() * 0.55 * (-t * 3.0).exp();
            dry.add(at + i, wash * 0.9 + boom, wash * 1.1 + boom);
        }
    }

    // The risers: white noise through a band sweeping up, over the four bars before each drop.
    for (from, to) in [(0, DROP1)] {
        let start = (from as f32 * 16.0 * step_samples) as usize;
        let span = ((to - from) as f32 * 16.0 * step_samples) as usize;
        let mut band = (0.0f32, 0.0f32);
        for i in 0..span {
            let progress = i as f32 / span as f32;
            let f = 300.0 * (40.0f32).powf(progress);
            let tune = 2.0 * (PI * f.min(16_000.0) / RATE).sin();
            band.0 += tune * band.1;
            let high = rng.next() - band.0 - 0.5 * band.1;
            band.1 += tune * high;
            let s = band.1 * 0.12 * progress * progress;
            dry.add(start + i, s * (1.0 - 0.3 * progress), s * (0.7 + 0.3 * progress));
        }
    }

    // The pads: seven detuned sawtooths a note, three notes, low-passed, slow in.
    {
        let detune = [-0.011f32, -0.007, -0.003, 0.0, 0.003, 0.007, 0.011];
        let mut phases = [[0.0f32; 7]; 3];
        for (n, row) in phases.iter_mut().enumerate() {
            for (k, p) in row.iter_mut().enumerate() {
                *p = ((n * 7 + k) as f32 * 0.137).fract();
            }
        }
        let mut lp = [0.0f32; 2];
        let mut lp2 = [0.0f32; 2];
        let mut level = 0.0f32;
        for i in 0..music_len + (RATE as usize * 3) {
            let bar = bar_of(i).min(BARS - 1);
            let notes = chord(bar);
            let ringing_out = bar >= END;
            let wanted = match bar {
                b if b < DROP1 => 0.7,
                b if b < END => 0.5,
                _ => 2.6,
            };
            let release = if i > music_len { (1.0 - (i - music_len) as f32 / (RATE * 3.0)).max(0.0) } else { 1.0 };
            level += (wanted * release - level) * (1.0 - (-1.0 / (0.6 * RATE)).exp());
            let mut l = 0.0;
            let mut r = 0.0;
            for (n, semis) in notes.iter().enumerate() {
                let base = hz(*semis as f32, 220.0);
                for (k, d) in detune.iter().enumerate() {
                    let f = base * (1.0 + d);
                    phases[n][k] = (phases[n][k] + f / RATE).fract();
                    let saw = 2.0 * phases[n][k] - 1.0;
                    let pan = k as f32 / 6.0;
                    l += saw * (1.0 - pan);
                    r += saw * pan;
                }
            }
            // Brighter in the breakdown and at the end.
            let open = if ringing_out { 3600.0 } else { 2400.0 };
            let g = 1.0 - (-TAU * open / RATE).exp();
            lp[0] += g * (l - lp[0]);
            lp[1] += g * (r - lp[1]);
            lp2[0] += g * (lp[0] - lp2[0]);
            lp2[1] += g * (lp[1] - lp2[1]);
            let s = 0.028 * level;
            pumped.add(i, lp2[0] * s, lp2[1] * s);
            echo_send.add(i, lp2[0] * s * 0.25, lp2[1] * s * 0.25);
        }
    }

    // The acid line, sample by sample, an octave above the bass.
    {
        let mut phase = 0.0f32;
        let mut pitch = 110.0f32;
        let mut target = 110.0f32;
        let mut stages = [0.0f32; 4];
        let mut envelope = 0.0f32;
        let mut accent = 0.0f32;
        let mut gate = 0.0f32;
        let mut sliding = false;
        let mut current = usize::MAX;
        for i in 0..music_len {
            let t = i as f32 / RATE;
            let step = (i as f32 / step_samples) as usize;
            let bar = step / 16;
            let playing = bar < END;
            let line = acid_pattern(bar >= DROP1 + 3);
            if step != current {
                current = step;
                let (note, accented, slide) = line[step % 16];
                match note {
                    Some(semis) if playing => {
                        target = hz((semis + root(bar).rem_euclid(12) * 0) as f32, 110.0);
                        if !sliding {
                            pitch = target;
                            envelope = 1.0;
                            accent = if accented { 1.0 } else { 0.0 };
                        }
                        gate = 1.0;
                    }
                    _ => gate = 0.0,
                }
                sliding = slide;
            }
            pitch += (target - pitch) * (1.0 - (-55.0 / RATE).exp());
            let within = (i as f32 / step_samples).fract();
            let held = if within < 0.6 || sliding { gate } else { 0.0 };
            envelope *= (-1.0 / (0.17 * RATE)).exp();
            accent *= (-1.0 / (0.1 * RATE)).exp();
            phase = (phase + pitch / RATE).fract();
            let saw = 2.0 * phase - 1.0;
            // Opening over each section: closed in the intro, opening through the first drop, wide
            // open through the second, breathing over four bars.
            let section = if bar < DROP1 {
                0.15 + 0.25 * t / (DROP1 as f32 * 16.0 * step_seconds)
            } else {
                (0.5 + 0.45 * (bar - DROP1) as f32 / (END - DROP1) as f32).min(0.95)
            };
            let breathe = 0.5 - 0.5 * (TAU * t / (4.0 * 16.0 * step_seconds)).cos();
            let cutoff = (150.0 + 2600.0 * section * (0.55 + 0.45 * breathe) + 2200.0 * envelope * section.max(0.3) + 2800.0 * accent).min(9000.0);
            let g = 1.0 - (-TAU * cutoff / RATE).exp();
            let resonance = 3.5 + 0.35 * accent;
            let input = saw - resonance * stages[3];
            stages[0] += g * (input.tanh() - stages[0]);
            stages[1] += g * (stages[0] - stages[1]);
            stages[2] += g * (stages[1] - stages[2]);
            stages[3] += g * (stages[2] - stages[3]);
            let amp = held * (0.6 + 0.4 * accent);
            let s = (stages[3] * 2.4).tanh() * amp * 0.32;
            pumped.add(i, s * 0.95, s * 1.05);
            echo_send.add(i, s * 0.12, s * 0.12);
        }
    }

    // The arpeggio: the chord's notes up and down in sixteenths, two octaves up, plucked.
    {
        let order = [0usize, 1, 2, 1, 0, 2, 1, 2];
        for step in 0..total_steps {
            let bar = step / 16;
            let on = (DROP1 + 1..END).contains(&bar);
            if !on {
                continue;
            }
            let notes = chord(bar);
            let k = order[step % order.len()];
            let octave = if step % 16 >= 8 { 880.0 } else { 440.0 };
            let f = hz(notes[k] as f32, octave);
            let at = (step as f32 * step_samples) as usize;
            let level = 0.075;
            let mut phase = 0.0f32;
            let mut lp = 0.0f32;
            for i in 0..(step_samples * 1.6) as usize {
                let t = i as f32 / RATE;
                phase = (phase + f / RATE).fract();
                let square = if phase < 0.5 { 1.0 } else { -1.0 };
                let saw = 2.0 * phase - 1.0;
                let raw = 0.5 * square + 0.5 * saw;
                let cutoff = 900.0 + 5000.0 * (-t * 25.0).exp();
                let g = 1.0 - (-TAU * cutoff / RATE).exp();
                lp += g * (raw - lp);
                let s = lp * (t / 0.002).min(1.0) * (-t * 9.0).exp() * level;
                let pan = if step % 2 == 0 { 0.8 } else { 1.2 };
                pumped.add(at + i, s * pan, s * (2.0 - pan));
                echo_send.add(at + i, s * 0.7, s * 0.7);
            }
        }
    }

    // The echo: a dotted eighth, crossing from side to side, darkening as it repeats.
    let delay = (3.0 * step_samples) as usize;
    let mut echo = Track { left: vec![0.0; length], right: vec![0.0; length] };
    let mut tone = [0.0f32; 2];
    for i in 0..length {
        let (back_l, back_r) = if i >= delay { (echo.right[i - delay], echo.left[i - delay]) } else { (0.0, 0.0) };
        let g = 1.0 - (-TAU * 3500.0 / RATE).exp();
        tone[0] += g * (back_l - tone[0]);
        tone[1] += g * (back_r - tone[1]);
        echo.left[i] = echo_send.left[i] + tone[0] * 0.45;
        echo.right[i] = echo_send.right[i] + tone[1] * 0.45;
    }

    // Mix: the pumped parts ducked under the kick, the echo, the dry parts; fade the very end,
    // soft clip, normalize just under full scale.
    let mut left = vec![0.0f32; length];
    let mut right = vec![0.0f32; length];
    let fade_from = music_len + (RATE * 1.0) as usize;
    for i in 0..length {
        let fade = if i > fade_from { (1.0 - (i - fade_from) as f32 / (RATE * 3.0)).max(0.0) } else { 1.0 };
        let l = dry.left[i] + (pumped.left[i] + echo.left[i] * 0.6) * duck[i];
        let r = dry.right[i] + (pumped.right[i] + echo.right[i] * 0.6) * duck[i];
        left[i] = (l * fade * 1.1).tanh();
        right[i] = (r * fade * 1.1).tanh();
    }
    let peak = left.iter().chain(&right).fold(0.0f32, |m, s| m.max(s.abs())).max(1e-6);
    let gain = 0.89 / peak;

    let mut file = BufWriter::new(File::create(&out).expect("the WAV can be created"));
    let data_len = (length * 4) as u32;
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_len).to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&2u16.to_le_bytes()).unwrap();
    file.write_all(&(RATE as u32).to_le_bytes()).unwrap();
    file.write_all(&(RATE as u32 * 4).to_le_bytes()).unwrap();
    file.write_all(&4u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_len.to_le_bytes()).unwrap();
    for i in 0..length {
        for s in [left[i], right[i]] {
            let v = (s * gain * 32767.0).clamp(-32768.0, 32767.0) as i16;
            file.write_all(&v.to_le_bytes()).unwrap();
        }
    }
    println!("{out}: {:.1} s at {BPM} BPM, {BARS} bars", length as f32 / RATE);
}

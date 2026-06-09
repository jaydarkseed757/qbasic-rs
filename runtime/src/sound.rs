/// QB-compatible sound: PLAY MML parser + SOUND/BEEP via rodio.

use rodio::{OutputStream, Sink};

// ── Note duration style ───────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum NoteStyle { Normal, Legato, Staccato }

// ── Persisted MML state (survives across PLAY calls) ─────────────────────────

#[derive(Clone)]
pub struct MmlState {
    pub octave:     i32,      // 0–6, default 4
    pub length:     i32,      // 1/length note, default 4
    pub tempo:      f64,      // BPM, default 120
    pub style:      NoteStyle,
    pub background: bool,     // MB = true, MF = false
}

impl Default for MmlState {
    fn default() -> Self {
        Self { octave: 4, length: 4, tempo: 120.0,
               style: NoteStyle::Normal, background: false }
    }
}

// ── A single sound event produced by the MML parser ──────────────────────────

#[derive(Clone)]
pub struct PlayEvent {
    freq_hz: f32,   // 0 = rest
    dur_ms:  u64,
}

// ── MML parser ────────────────────────────────────────────────────────────────

/// Parse a QB PLAY MML string into a list of events, updating `state` in place.
pub fn parse_mml(mml: &str, state: &mut MmlState) -> Vec<PlayEvent> {
    let chars: Vec<char> = mml.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut events = Vec::new();

    // Helper: read a decimal integer starting at position i.
    // Returns (value, new_i). Returns (0, i) if no digits present.
    fn read_int(chars: &[char], mut i: usize) -> (i32, usize) {
        let mut v: i32 = 0;
        let mut any = false;
        while i < chars.len() && chars[i].is_ascii_digit() {
            v = v * 10 + (chars[i] as i32 - '0' as i32);
            i += 1;
            any = true;
        }
        if any { (v, i) } else { (0, i) }
    }

    // Quarter-note duration in ms given current tempo.
    let quarter_ms = |tempo: f64| -> f64 { 60_000.0 / tempo };

    // Duration for a given length value (1=whole, 4=quarter, …).
    // Dots: each trailing '.' multiplies by 1.5^n cumulatively.
    let note_dur_ms = |length: i32, dots: u32, tempo: f64| -> u64 {
        let mut ms = quarter_ms(tempo) * 4.0 / (length as f64);
        let mut dot_add = ms / 2.0;
        for _ in 0..dots {
            ms += dot_add;
            dot_add /= 2.0;
        }
        ms as u64
    };

    // Style fraction: how much of the note slot is actually sounding.
    let style_frac = |s: NoteStyle| -> f64 {
        match s {
            NoteStyle::Normal   => 7.0 / 8.0,
            NoteStyle::Legato   => 1.0,
            NoteStyle::Staccato => 3.0 / 4.0,
        }
    };

    while i < len {
        let ch = chars[i].to_ascii_uppercase();
        i += 1;

        match ch {
            // ── Mode/style ────────────────────────────────────────────────────
            'M' => {
                if i < len {
                    match chars[i].to_ascii_uppercase() {
                        'B' => { state.background = true;  i += 1; }
                        'F' => { state.background = false; i += 1; }
                        'N' => { state.style = NoteStyle::Normal;   i += 1; }
                        'L' => { state.style = NoteStyle::Legato;   i += 1; }
                        'S' => { state.style = NoteStyle::Staccato; i += 1; }
                        _   => {}
                    }
                }
            }

            // ── Octave ────────────────────────────────────────────────────────
            'O' => {
                let (v, ni) = read_int(&chars, i); i = ni;
                state.octave = v.max(0).min(6);
            }
            '<' => { state.octave = (state.octave - 1).max(0); }
            '>' => { state.octave = (state.octave + 1).min(6); }

            // ── Length ────────────────────────────────────────────────────────
            'L' => {
                let (v, ni) = read_int(&chars, i); i = ni;
                if v > 0 { state.length = v; }
            }

            // ── Tempo ─────────────────────────────────────────────────────────
            'T' => {
                let (v, ni) = read_int(&chars, i); i = ni;
                state.tempo = (v as f64).max(32.0).min(255.0);
            }

            // ── Rest / pause ──────────────────────────────────────────────────
            'P' => {
                let (v, ni) = read_int(&chars, i); i = ni;
                let length = if v > 0 { v } else { state.length };
                // Count dots
                let mut dots = 0u32;
                while i < len && chars[i] == '.' { dots += 1; i += 1; }
                events.push(PlayEvent {
                    freq_hz: 0.0,
                    dur_ms: note_dur_ms(length, dots, state.tempo),
                });
            }

            // ── Note by number ────────────────────────────────────────────────
            'N' => {
                let (v, ni) = read_int(&chars, i); i = ni;
                if v == 0 {
                    // N0 = rest for current length
                    events.push(PlayEvent { freq_hz: 0.0,
                        dur_ms: note_dur_ms(state.length, 0, state.tempo) });
                } else {
                    // N1-N84: absolute note number
                    let total_ms = note_dur_ms(state.length, 0, state.tempo);
                    let sound_ms = (total_ms as f64 * style_frac(state.style)) as u64;
                    let rest_ms  = total_ms - sound_ms;
                    events.push(PlayEvent { freq_hz: note_num_to_freq(v), dur_ms: sound_ms });
                    if rest_ms > 0 {
                        events.push(PlayEvent { freq_hz: 0.0, dur_ms: rest_ms });
                    }
                }
            }

            // ── Regular notes A–G ─────────────────────────────────────────────
            'A'|'B'|'C'|'D'|'E'|'F'|'G' => {
                // Note name → semitone offset from C
                let mut semitone: i32 = match ch {
                    'C' => 0, 'D' => 2, 'E' => 4, 'F' => 5,
                    'G' => 7, 'A' => 9, 'B' => 11, _ => 0,
                };

                // Optional accidental: + or # = sharp, - = flat
                if i < len && (chars[i] == '+' || chars[i] == '#') {
                    semitone += 1; i += 1;
                } else if i < len && chars[i] == '-' {
                    semitone -= 1; i += 1;
                }

                // Optional explicit length
                let (v, ni) = read_int(&chars, i); i = ni;
                let length = if v > 0 { v } else { state.length };

                // Count dots
                let mut dots = 0u32;
                while i < len && chars[i] == '.' { dots += 1; i += 1; }

                let freq = semitone_to_freq(state.octave, semitone);
                let total_ms = note_dur_ms(length, dots, state.tempo);
                let sound_ms = (total_ms as f64 * style_frac(state.style)) as u64;
                let rest_ms  = total_ms - sound_ms;

                events.push(PlayEvent { freq_hz: freq, dur_ms: sound_ms.max(1) });
                if rest_ms > 0 {
                    events.push(PlayEvent { freq_hz: 0.0, dur_ms: rest_ms });
                }
            }

            // ── Whitespace and separators ─────────────────────────────────────
            ' ' | ';' | '\t' => {}

            _ => {} // ignore unknown
        }
    }

    events
}

// ── Frequency helpers ─────────────────────────────────────────────────────────

/// Semitone offset from C (0=C, 1=C#, …, 11=B) in given QB octave → Hz.
/// QB octave 3: middle C (O3 C = 261.63 Hz), concert A (O3 A = 440 Hz).
/// N37 = middle C = O3 C; N46 = O3 A = 440 Hz. O4 A = 880 Hz.
fn semitone_to_freq(octave: i32, semitone: i32) -> f32 {
    // A440 lives at QB O3 A (octave=3, semitone=9 → abs=45)
    let a440_abs = 3 * 12 + 9; // 45
    let abs_semitone = octave * 12 + semitone;
    440.0_f32 * 2.0_f32.powf((abs_semitone - a440_abs) as f32 / 12.0)
}

/// QB PLAY N command: absolute note number 1–84 → Hz.
/// N1 = O0 C (32.7 Hz), N37 = middle C (261.63 Hz), N46 = A440.
fn note_num_to_freq(n: i32) -> f32 {
    // N46 = O3 A = 440 Hz (concert A in QB octave 3)
    440.0_f32 * 2.0_f32.powf((n - 46) as f32 / 12.0)
}

// ── Playback ──────────────────────────────────────────────────────────────────

/// Synthesize a sequence of notes as a single f32 PCM buffer at 44100 Hz.
fn events_to_pcm(events: &[PlayEvent]) -> Vec<f32> {
    const SAMPLE_RATE: f64 = 44_100.0;
    let mut samples = Vec::new();
    let mut phase = 0.0_f64;

    for ev in events {
        let n_samples = ((ev.dur_ms as f64 / 1000.0) * SAMPLE_RATE) as usize;
        if ev.freq_hz <= 0.0 || ev.freq_hz > 20_000.0 {
            // Rest
            for _ in 0..n_samples { samples.push(0.0_f32); }
        } else {
            let phase_step = ev.freq_hz as f64 / SAMPLE_RATE;
            // Simple fade-in/out (8 ms each) to avoid clicks
            let fade = ((0.008 * SAMPLE_RATE) as usize).min(n_samples / 4);
            for s in 0..n_samples {
                let amp = if s < fade {
                    s as f32 / fade as f32
                } else if s >= n_samples - fade {
                    (n_samples - s) as f32 / fade as f32
                } else {
                    1.0
                };
                // Sawtooth wave: harmonically rich like the original PC speaker.
                // A 65 Hz sawtooth has audible harmonics at 130, 260, 520 Hz even
                // when the fundamental is below laptop-speaker cutoff (~200 Hz).
                let sample = (phase * 2.0 - 1.0) as f32;
                samples.push(sample * amp * 0.20); // slightly lower amp than sine (sawtooth is louder)
                phase = (phase + phase_step).fract();
            }
        }
    }
    samples
}

// ── Public API called by Runtime ──────────────────────────────────────────────

/// Play a sequence of events synchronously (blocks until done).
pub fn play_events_blocking(events: &[PlayEvent]) {
    if events.is_empty() { return; }
    let pcm = events_to_pcm(events);
    if pcm.is_empty() { return; }
    let _ = play_pcm_blocking(pcm);
}

fn play_pcm_blocking(pcm: Vec<f32>) -> Result<(), Box<dyn std::error::Error>> {
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;
    let source = rodio::buffer::SamplesBuffer::new(1, 44_100, pcm);
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}

/// Play a sequence of events in a background thread (non-blocking).
pub fn play_events_background(events: Vec<PlayEvent>) {
    if events.is_empty() { return; }
    let pcm = events_to_pcm(&events);
    if pcm.is_empty() { return; }
    std::thread::spawn(move || {
        let _ = play_pcm_blocking(pcm);
    });
}

/// SOUND statement: freq Hz for duration/18.2 seconds.
pub fn play_sound(freq: f64, duration_ticks: f64) {
    if freq < 37.0 || freq > 32_767.0 || duration_ticks <= 0.0 { return; }
    let dur_ms = ((duration_ticks / 18.2) * 1000.0).min(60_000.0) as u64;
    let events = vec![PlayEvent { freq_hz: freq as f32, dur_ms }];
    play_events_blocking(&events);
}

/// BEEP statement.
pub fn play_beep() {
    play_sound(800.0, 4.0); // ~220 ms at 800 Hz
}

// ── Test helpers (no audio device required) ───────────────────────────────────

/// Exposed for integration tests: parse MML without playing audio.
/// Returns Vec<(freq_hz, dur_ms)>.
pub fn parse_mml_for_test(mml: &str, state: &mut MmlState) -> Vec<(f32, u64)> {
    parse_mml(mml, state)
        .into_iter()
        .map(|e| (e.freq_hz, e.dur_ms))
        .collect()
}

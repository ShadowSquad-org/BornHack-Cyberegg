pub mod melodies;

use embassy_nrf::gpio::Output;
use embassy_time::Timer;

/// Musical note (equal temperament, A4 = 440 Hz)
#[derive(Clone, Copy, PartialEq)]
pub enum Note {
    // Octave 3  (131–247 Hz)
    C3, Cs3, D3, Ds3, E3, F3, Fs3, G3, Gs3, A3, As3, B3,
    // Octave 4  (262–494 Hz)
    C4, Cs4, D4, Ds4, E4, F4, Fs4, G4, Gs4, A4, As4, B4,
    /// Silence
    Rest,
}

impl Note {
    pub const fn freq_hz(self) -> u32 {
        match self {
            Note::C3  => 131, Note::Cs3 => 139, Note::D3  => 147,
            Note::Ds3 => 156, Note::E3  => 165, Note::F3  => 175,
            Note::Fs3 => 185, Note::G3  => 196, Note::Gs3 => 208,
            Note::A3  => 220, Note::As3 => 233, Note::B3  => 247,
            Note::C4  => 262, Note::Cs4 => 277, Note::D4  => 294,
            Note::Ds4 => 311, Note::E4  => 330, Note::F4  => 349,
            Note::Fs4 => 370, Note::G4  => 392, Note::Gs4 => 415,
            Note::A4  => 440, Note::As4 => 466, Note::B4  => 494,
            Note::Rest => 0,
        }
    }
}

/// A single step in a melody: a note and how long to play it (ms)
#[derive(Clone, Copy)]
pub struct Tone {
    pub note: Note,
    pub duration_ms: u32,
}

impl Tone {
    pub const fn new(note: Note, duration_ms: u32) -> Self {
        Self { note, duration_ms }
    }
}

/// Convenience shorthand: `tone!(A4, 200)` → `Tone::new(Note::A4, 200)`
#[macro_export]
macro_rules! tone {
    ($note:ident, $ms:expr) => {
        $crate::fw::buzzer::Tone::new($crate::fw::buzzer::Note::$note, $ms)
    };
}

pub struct Buzzer<'a> {
    pin: Output<'a>,
}

impl<'a> Buzzer<'a> {
    pub fn new(pin: Output<'a>) -> Self {
        Self { pin }
    }

    /// Play a raw frequency (Hz) for a duration (ms). freq_hz = 0 is silence.
    pub async fn play_freq(&mut self, freq_hz: u32, duration_ms: u32) {
        if freq_hz == 0 || duration_ms == 0 {
            Timer::after_millis(duration_ms as u64).await;
            return;
        }
        let half_period_us = 500_000u32 / freq_hz;
        let cycles = (freq_hz as u64 * duration_ms as u64) / 1000;
        for _ in 0..cycles {
            self.pin.set_high();
            Timer::after_micros(half_period_us as u64).await;
            self.pin.set_low();
            Timer::after_micros(half_period_us as u64).await;
        }
    }

    /// Play a single Tone. Use `Note::Rest` for explicit pauses between notes.
    pub async fn play(&mut self, tone: Tone) {
        self.play_freq(tone.note.freq_hz(), tone.duration_ms).await;
    }

    /// Play a sequence of Tones in order.
    pub async fn play_melody(&mut self, melody: &[Tone]) {
        for &tone in melody {
            self.play(tone).await;
        }
    }
}

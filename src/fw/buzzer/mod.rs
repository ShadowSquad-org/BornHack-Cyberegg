pub mod melodies;

use embassy_nrf::pwm::{DutyCycle, SimplePwm};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;

/// All available melodies, addressable by index.
pub const MELODIES: &[&[Tone]] = &[
    melodies::STARTUP,        // 0
    melodies::RICK_INTRO,     // 1
    melodies::IMPERIAL_MARCH, // 2
];

/// Signal a melody index to the buzzer task.
/// If a melody is already playing it will be interrupted at the next note boundary.
static MELODY_SIGNAL: Signal<CriticalSectionRawMutex, usize> = Signal::new();

/// Trigger melody `index` (see [`MELODIES`]) without blocking the caller.
/// Out-of-range indices are silently ignored.
pub fn play(index: usize) {
    if index < MELODIES.len() {
        MELODY_SIGNAL.signal(index);
    }
}

/// Embassy task that owns the buzzer and plays melodies on demand.
/// Spawn once from `main`; use [`play`] to trigger melodies from anywhere.
#[embassy_executor::task]
pub async fn buzzer_task(mut buzzer: Buzzer<'static>) {
    loop {
        let index = MELODY_SIGNAL.wait().await;
        if let Some(melody) = MELODIES.get(index) {
            for &tone in *melody {
                // A new melody arrived — finish this tone then switch.
                buzzer.play(tone).await;
                if MELODY_SIGNAL.signaled() {
                    break;
                }
            }
            buzzer.pwm.disable();
        }
    }
}

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

/// PWM-driven passive buzzer.
///
/// The PWM peripheral generates the tone waveform autonomously — no
/// per-half-period timer wakes needed. For each note `set_period` loads the
/// correct COUNTERTOP, a 50% `DutyCycle` is set, and `enable()`/`disable()`
/// bookend the `Timer::after_millis` wait. The idle pin level is LOW
/// (configured via [`SimpleConfig`]), so silence and rests keep the pin low.
pub struct Buzzer<'d> {
    pwm: SimplePwm<'d>,
}

impl<'d> Buzzer<'d> {
    /// Take ownership of a [`SimplePwm`] configured for buzzer use.
    pub fn new(pwm: SimplePwm<'d>) -> Self {
        // PWM starts disabled; idle level is LOW (SimpleConfig default).
        Self { pwm }
    }

    /// Play `freq_hz` for `duration_ms` milliseconds. `freq_hz = 0` is silence.
    pub async fn play_freq(&mut self, freq_hz: u32, duration_ms: u32) {
        if freq_hz == 0 {
            // Rest: pin stays LOW (PWM disabled), just wait.
            Timer::after_millis(duration_ms as u64).await;
            return;
        }

        // set_period computes COUNTERTOP = PWM_CLK / freq (default Div16 → 1 MHz base).
        self.pwm.set_period(freq_hz);
        // Enable before set_duty so that sync_duty_cyles_to_peripheral fires SEQSTART
        // while the peripheral is enabled — it then waits for SEQEND, guaranteeing the
        // waveform is loaded before the timer starts.  (new_inner already enables, but
        // disable() in a previous note turns it off.)
        self.pwm.enable();
        // 50% square wave: duty = COUNTERTOP / 2.
        // DutyCycle::normal(v): output HIGH when counter >= v.
        let duty = DutyCycle::normal(self.pwm.max_duty() / 2);
        self.pwm.set_duty(0, duty);

        Timer::after_millis(duration_ms as u64).await;

        self.pwm.disable(); // pin returns to idle LOW
    }

    /// Play a single [`Tone`].
    pub async fn play(&mut self, tone: Tone) {
        self.play_freq(tone.note.freq_hz(), tone.duration_ms).await;
    }

    /// Play a slice of [`Tone`]s in order.
    pub async fn play_melody(&mut self, melody: &[Tone]) {
        for &tone in melody {
            self.play(tone).await;
        }
    }
}

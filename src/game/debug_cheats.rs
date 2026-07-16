//! Hidden debug/cheat sequence.
//!
//! Lets a developer jump straight to the weight/diabetes states for
//! testing instead of waiting out the real multi-day arc. Entered by
//! pressing a fixed button sequence while idle on the main game screen
//! (no modal or mini-game open) — same "hidden combo" idea as the
//! existing boot-time combos (DFU, safe-LUT, factory reset), just
//! entered as a sequence during play instead of held at boot.
//!
//! Sequence: Up, Up, Down, Down, Left, Right, Left, Right, Fire — the
//! classic template, adapted to this badge's Up/Down/Left/Right/Fire
//! button set (no A/B/Start).

use crate::menu::ButtonId;
use core::sync::atomic::{AtomicU8, Ordering};

const SEQUENCE: [ButtonId; 9] = [
    ButtonId::Up,
    ButtonId::Up,
    ButtonId::Down,
    ButtonId::Down,
    ButtonId::Left,
    ButtonId::Right,
    ButtonId::Left,
    ButtonId::Right,
    ButtonId::Fire,
];

/// How many correct presses in a row have been made so far.
static PROGRESS: AtomicU8 = AtomicU8::new(0);

/// Feed one button press into the sequence tracker.
///
/// Returns `true` the instant the full sequence completes — the caller
/// should open the debug menu and treat the triggering press as
/// consumed (not also forwarded to normal navigation). Returns `false`
/// otherwise, including every partial-progress press, so the caller can
/// fall through to its regular button handling.
pub fn feed(btn: ButtonId) -> bool {
    let progress = PROGRESS.load(Ordering::Relaxed) as usize;
    if btn == SEQUENCE[progress] {
        let next = progress + 1;
        if next == SEQUENCE.len() {
            PROGRESS.store(0, Ordering::Relaxed);
            return true;
        }
        PROGRESS.store(next as u8, Ordering::Relaxed);
    } else {
        // Wrong button — restart, but let this press count as step one
        // if it happens to match the sequence's first button, so
        // slightly-mistimed re-attempts don't have to fully pause first.
        let restart = if btn == SEQUENCE[0] { 1 } else { 0 };
        PROGRESS.store(restart, Ordering::Relaxed);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_sequence_returns_true_once() {
        let mut result = false;
        for &btn in &SEQUENCE {
            result = feed(btn);
        }
        assert!(result, "the final press of the correct sequence should return true");
    }

    #[test]
    fn wrong_button_resets_progress() {
        assert!(!feed(ButtonId::Up));
        assert!(!feed(ButtonId::Up));
        // Wrong button here — breaks the sequence.
        assert!(!feed(ButtonId::Fire));
        // Now replay the whole thing; should still need all 9 presses.
        for &btn in &SEQUENCE[..SEQUENCE.len() - 1] {
            assert!(!feed(btn));
        }
        assert!(feed(*SEQUENCE.last().unwrap()));
    }

    #[test]
    fn partial_progress_never_returns_true_early() {
        for &btn in &SEQUENCE[..SEQUENCE.len() - 1] {
            assert!(!feed(btn));
        }
    }
}

//! BornPets icon-grid navigation state.
//!
//! The game screen has an 8-icon grid: 4 icons on the top row and 4 on the
//! bottom row.  `GameNav` tracks which icon is focused and exposes one method
//! per joystick direction.  The state is stored globally as a packed `AtomicU8`
//! so it can be updated from the button ISR and read from the render task
//! without a blocking mutex.
//!
//! # Bit layout of `NAV_STATE`
//! ```text
//! bit 2 : row   (0 = Top, 1 = Bottom)
//! bits 1-0 : col (0–3)
//! ```
//! Default: `0b111` → bottom row, col 3 (the twofaces icon).

use core::sync::atomic::{AtomicU8, Ordering};

// ── Types ─────────────────────────────────────────────────────────────────────

/// Which icon row is focused.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Top,
    Bottom,
}

/// Cursor position within the 2×4 game icon grid.
#[derive(Clone, Copy)]
pub struct GameNav {
    pub row: Row,
    pub col: u8,
}

/// Outcome of a [`nav_right`] call.
pub enum NavResult {
    /// Cursor moved within the grid.
    Moved,
    /// Cursor was already at the rightmost column — caller should switch screen.
    NextScreen,
}

// ── Global packed state ───────────────────────────────────────────────────────

/// Packed nav: bit 2 = row, bits 1–0 = col.  Default = bottom-right (7).
static NAV_STATE: AtomicU8 = AtomicU8::new(0b111);

fn pack(nav: &GameNav) -> u8 {
    ((nav.row == Row::Bottom) as u8) << 2 | (nav.col & 0b11)
}

fn unpack(v: u8) -> GameNav {
    GameNav {
        row: if v & 0b100 != 0 { Row::Bottom } else { Row::Top },
        col: v & 0b11,
    }
}

/// Read the current navigation state.
pub fn get_nav() -> GameNav {
    unpack(NAV_STATE.load(Ordering::Relaxed))
}

// ── Public nav actions (called from button handler) ───────────────────────────

/// Move focus to the top icon row (same column).
pub fn nav_up() {
    let mut n = unpack(NAV_STATE.load(Ordering::Relaxed));
    n.row = Row::Top;
    NAV_STATE.store(pack(&n), Ordering::Relaxed);
}

/// Move focus to the bottom icon row (same column).
pub fn nav_down() {
    let mut n = unpack(NAV_STATE.load(Ordering::Relaxed));
    n.row = Row::Bottom;
    NAV_STATE.store(pack(&n), Ordering::Relaxed);
}

/// Move focus one column to the left (clamps at column 0).
pub fn nav_left() {
    let mut n = unpack(NAV_STATE.load(Ordering::Relaxed));
    if n.col > 0 {
        n.col -= 1;
    }
    NAV_STATE.store(pack(&n), Ordering::Relaxed);
}

/// Move focus one column to the right.
///
/// Returns [`NavResult::NextScreen`] when already at the rightmost column;
/// the caller is then responsible for switching to the next screen.
pub fn nav_right() -> NavResult {
    let mut n = unpack(NAV_STATE.load(Ordering::Relaxed));
    if n.col < 3 {
        n.col += 1;
        NAV_STATE.store(pack(&n), Ordering::Relaxed);
        NavResult::Moved
    } else {
        NavResult::NextScreen
    }
}

//! BornPets game-screen input routing.
//!
//! [`dispatch`] is the single entry point for all button events while the game
//! screen is active.  It decides whether a modal is open or not, and routes
//! accordingly.  Returning `false` tells the caller that the event was not
//! consumed by the game layer and should be forwarded to the menu.

use crate::menu::ButtonId;
use super::modal;
use super::nav::{get_nav, nav_down, nav_left, nav_right, nav_up, NavResult};

/// Route a button press on the game screen.
///
/// Returns `true` if the game layer consumed the event.
/// Returns `false` if the caller should forward to the menu
/// (e.g. `Right` at the grid edge to advance to the next screen).
pub fn dispatch(btn: ButtonId) -> bool {
    if modal::is_open() {
        match btn {
            ButtonId::Cancel             => modal::close(),
            ButtonId::Up                 => modal::cursor_up(),
            ButtonId::Down               => modal::cursor_down(),
            ButtonId::Fire | ButtonId::Execute => modal::activate(),
            ButtonId::Left | ButtonId::Right   => {}
        }
        true
    } else {
        match btn {
            ButtonId::Up    => { nav_up();   true }
            ButtonId::Down  => { nav_down(); true }
            ButtonId::Left  => { nav_left(); true }
            ButtonId::Right => matches!(nav_right(), NavResult::Moved),
            ButtonId::Fire | ButtonId::Execute => {
                let nav  = get_nav();
                let kind = modal::kind_for_icon(nav.row, nav.col);
                modal::open(kind);
                true
            }
            ButtonId::Cancel => true,
        }
    }
}

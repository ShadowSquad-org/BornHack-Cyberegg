//! Tiny screen-agnostic notification banner.  Use it to give the user
//! immediate feedback for actions that otherwise leave no on-screen
//! trace — e.g. "added Quick test at 19:42" after a Settings → Events
//! → Quick test, where the work happens silently in alarm-slot RAM and
//! the user is left looking at the same menu they fired from.
//!
//! Behaviour: a single global slot for one message at a time.  Calling
//! [`show`] replaces whatever was there.  The renderer overlays a
//! BLACK-filled banner with WHITE text across the bottom of the screen
//! whenever a message is set.  Any subsequent button press dismisses
//! the banner — the dispatcher consumes the press as the dismissal
//! (so you don't accidentally fire the menu item underneath).
//!
//! Same shape as `text_entry`: a `Mutex<RefCell<Option<...>>>` static,
//! plus `show` / `dismiss` / `is_active` / `draw_overlay` helpers.

use core::cell::RefCell;

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Alignment, Baseline, Text, TextStyleBuilder};

use crate::{BLACK, TriColor, WHITE};

/// Maximum length of one toast message.  A bit larger than menu labels
/// so we can fit `"Added 'Bevrijdingsfestival' 19:42"` worth of text.
pub const TOAST_LEN: usize = 48;

#[cfg(feature = "embassy-base")]
pub static TOAST: embassy_sync::blocking_mutex::Mutex<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    RefCell<Option<heapless::String<TOAST_LEN>>>,
> = embassy_sync::blocking_mutex::Mutex::new(RefCell::new(None));

#[cfg(feature = "simulator")]
pub static TOAST: std::sync::Mutex<RefCell<Option<heapless::String<TOAST_LEN>>>> =
    std::sync::Mutex::new(RefCell::new(None));

/// Replace the current toast (if any) with `msg`.  Truncates silently
/// if the message exceeds `TOAST_LEN`.
pub fn show(msg: &str) {
    let mut s: heapless::String<TOAST_LEN> = heapless::String::new();
    for ch in msg.chars() {
        if s.push(ch).is_err() {
            break;
        }
    }
    #[cfg(feature = "embassy-base")]
    TOAST.lock(|cell| cell.replace(Some(s)));
    #[cfg(feature = "simulator")]
    TOAST.lock().unwrap().replace(Some(s));
}

/// Clear any pending toast.  Called by the dispatcher when consuming
/// the dismissal button press.
pub fn dismiss() {
    #[cfg(feature = "embassy-base")]
    TOAST.lock(|cell| cell.replace(None));
    #[cfg(feature = "simulator")]
    TOAST.lock().unwrap().replace(None);
}

pub fn is_active() -> bool {
    #[cfg(feature = "embassy-base")]
    return TOAST.lock(|cell| cell.borrow().is_some());
    #[cfg(feature = "simulator")]
    return TOAST.lock().unwrap().borrow().is_some();
}

/// Draw the banner across the bottom 18 px of the display.  Caller is
/// expected to invoke this *after* the active screen has rendered, so
/// the banner ends up on top.
pub fn draw_overlay<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    let msg: Option<heapless::String<TOAST_LEN>> = {
        #[cfg(feature = "embassy-base")]
        {
            TOAST.lock(|cell| cell.borrow().clone())
        }
        #[cfg(feature = "simulator")]
        {
            TOAST.lock().unwrap().borrow().clone()
        }
    };
    let Some(msg) = msg else {
        return Ok(());
    };

    // Banner: BLACK rectangle along the bottom of the 152×152 panel,
    // WHITE text centred inside it.  18 px tall = enough for one
    // FONT_6X10 line plus ~4 px of padding.
    Rectangle::new(Point::new(0, 134), Size::new(152, 18))
        .into_styled(PrimitiveStyle::with_fill(BLACK))
        .draw(display)?;

    let centered = TextStyleBuilder::new()
        .baseline(Baseline::Middle)
        .alignment(Alignment::Center)
        .build();
    Text::with_text_style(
        msg.as_str(),
        Point::new(76, 143),
        MonoTextStyle::new(&FONT_6X10, WHITE),
        centered,
    )
    .draw(display)?;
    Ok(())
}

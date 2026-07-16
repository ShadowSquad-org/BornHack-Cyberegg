//! Friends screen — pets met on the mesh "SHDW" channel.
//!
//! A full-screen overlay opened from the Stats modal, same pattern as
//! `realm_view` (Unicorn Realm): scrollable list, Up/Down to scroll,
//! any other button closes it.

use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Baseline, Text, TextStyleBuilder};

use crate::ui::{self, TEXT_BLACK, TEXT_BOLD_BLACK};
use crate::{BLACK, TriColor, WHITE};

static ACTIVE: AtomicBool = AtomicBool::new(false);
static SCROLL: AtomicU8 = AtomicU8::new(0);

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

pub fn open() {
    SCROLL.store(0, Ordering::Relaxed);
    ACTIVE.store(true, Ordering::Relaxed);
}

pub fn close() {
    ACTIVE.store(false, Ordering::Relaxed);
}

pub fn scroll_up() {
    let s = SCROLL.load(Ordering::Relaxed);
    if s > 0 {
        SCROLL.store(s - 1, Ordering::Relaxed);
    }
}

pub fn scroll_down() {
    let count = super::friends::count();
    let s = SCROLL.load(Ordering::Relaxed);
    if s + 1 < count {
        SCROLL.store(s + 1, Ordering::Relaxed);
    }
}

/// Format ticks-since-first-met as "Xd Xh" (1 tick = 10s, same convention
/// as `PetRecord::age_str`).
fn since_str(ticks: u32) -> heapless::String<12> {
    let hours = ticks / 360;
    let days = hours / 24;
    let mut s = heapless::String::new();
    let _ = core::fmt::Write::write_fmt(&mut s, format_args!("{}d {}h", days, hours % 24));
    s
}

pub fn draw<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    let count = super::friends::count();
    let scroll = SCROLL.load(Ordering::Relaxed) as usize;
    let now = super::lifecycle::now_tick();

    // Background.
    Rectangle::new(Point::zero(), Size::new(152, 152))
        .into_styled(PrimitiveStyle::with_fill(WHITE))
        .draw(display)?;

    // Title bar.
    ui::draw_title_bar(display, "Friends", Point::zero(), 152)?;

    if count == 0 {
        ui::draw_centered_message(display, "No friends met yet", Point::new(76, 85))?;
        return Ok(());
    }

    let left = TextStyleBuilder::new().baseline(Baseline::Top).build();

    // Show up to 4 friends per screen.
    let visible = 4usize.min(count as usize - scroll);
    for i in 0..visible {
        let idx = scroll + i;
        let Some(friend) = super::friends::get(idx) else {
            break;
        };

        let y = 22 + i as i32 * 32;

        let mut line: heapless::String<28> = heapless::String::new();
        let name = friend.name_str();
        let kind_name = super::engine::PetKind::from_u8(friend.pet_kind).name();
        let since = since_str(now.saturating_sub(friend.first_seen_tick));
        if !name.is_empty() {
            let _ = core::fmt::Write::write_fmt(
                &mut line,
                format_args!("{} ({}) - {}", name, kind_name, since),
            );
        } else {
            let _ =
                core::fmt::Write::write_fmt(&mut line, format_args!("{} - {}", kind_name, since));
        }
        Text::with_text_style(line.as_str(), Point::new(4, y), TEXT_BOLD_BLACK, left)
            .draw(display)?;

        let recently_boosted = now.saturating_sub(friend.last_boost_tick)
            < super::friends::FRIEND_BOOST_COOLDOWN_TICKS;
        let sub = if recently_boosted {
            "Spent time together recently"
        } else {
            "Known friend"
        };
        Text::with_text_style(sub, Point::new(4, y + 14), TEXT_BLACK, left).draw(display)?;

        if i + 1 < visible {
            Rectangle::new(Point::new(4, y + 29), Size::new(144, 1))
                .into_styled(PrimitiveStyle::with_fill(BLACK))
                .draw(display)?;
        }
    }

    // Scroll indicator.
    if count as usize > 4 {
        let mut indicator: heapless::String<8> = heapless::String::new();
        let _ =
            core::fmt::Write::write_fmt(&mut indicator, format_args!("{}/{}", scroll + 1, count));
        let right = TextStyleBuilder::new()
            .baseline(Baseline::Bottom)
            .alignment(embedded_graphics::text::Alignment::Right)
            .build();
        Text::with_text_style(indicator.as_str(), Point::new(148, 150), TEXT_BLACK, right)
            .draw(display)?;
    }

    Ok(())
}

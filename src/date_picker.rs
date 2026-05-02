//! Full-screen date / time picker overlay.  Same shape as `text_entry`:
//! a global slot, `begin`/`is_active`/`dismiss`/`dispatch`/`draw_overlay`
//! helpers, and a callback-on-submit pattern.
//!
//! Layout (152×152):
//!
//! ```text
//!     <Title>
//!  Year:    2026     <- row 0
//!  Month:   05       <- row 1
//!  Day:     02       <- row 2
//!  Hour:    19       <- row 3
//!  Minute:  30       <- row 4
//!  ←/→ value   Fire=save
//! ```
//!
//! The active row gets a black background with white text (same idiom as
//! the menu's centre row).  Buttons:
//!
//!   * Up / Down    — move the field cursor
//!   * Left / Right — dec / inc the active field's value (with wrap, and
//!                    day clamped to the chosen month's length so you
//!                    can't pick Feb 30)
//!   * Fire / Exec  — call `on_complete(year, month, day, hour, minute)`
//!                    and dismiss
//!   * Cancel       — dismiss without calling the callback

use core::cell::RefCell;

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::{FONT_6X10, FONT_7X13_BOLD};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Alignment, Baseline, Text, TextStyleBuilder};

use crate::menu::ButtonId;
use crate::{BLACK, TriColor, WHITE};

const FIELD_COUNT: u8 = 5;
const F_YEAR: u8 = 0;
const F_MONTH: u8 = 1;
const F_DAY: u8 = 2;
const F_HOUR: u8 = 3;
const F_MINUTE: u8 = 4;

// Layout
const ROW_TOP_Y: i32 = 30;
const ROW_H: i32 = 18;
const ROW_W: u32 = 152;

pub struct DatePicker {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    field: u8,
    title: &'static str,
    on_complete: fn(u16, u8, u8, u8, u8),
}

impl DatePicker {
    pub fn new(
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        on_complete: fn(u16, u8, u8, u8, u8),
        title: &'static str,
    ) -> Self {
        Self {
            year,
            month: month.clamp(1, 12),
            day: day.clamp(1, days_in_month(year, month.clamp(1, 12))),
            hour: hour.min(23),
            minute: minute.min(59),
            field: F_YEAR,
            title,
            on_complete,
        }
    }

    /// Returns true when the picker is done (submitted or cancelled).
    pub fn dispatch(&mut self, btn: ButtonId) -> bool {
        match btn {
            ButtonId::Up => {
                if self.field > 0 {
                    self.field -= 1;
                }
            }
            ButtonId::Down => {
                if self.field < FIELD_COUNT - 1 {
                    self.field += 1;
                }
            }
            ButtonId::Right => self.step(true),
            ButtonId::Left => self.step(false),
            ButtonId::Fire | ButtonId::Execute => {
                (self.on_complete)(self.year, self.month, self.day, self.hour, self.minute);
                return true;
            }
            ButtonId::Cancel => return true,
        }
        false
    }

    fn step(&mut self, up: bool) {
        match self.field {
            F_YEAR => {
                // Clamp range — wrap at 1900..2099 just to keep things sane.
                if up && self.year < 2099 {
                    self.year += 1;
                } else if !up && self.year > 1970 {
                    self.year -= 1;
                }
                // Re-clamp day in case Feb 29 → Feb 28 across leap years.
                let dim = days_in_month(self.year, self.month);
                if self.day > dim {
                    self.day = dim;
                }
            }
            F_MONTH => {
                self.month = if up {
                    if self.month >= 12 { 1 } else { self.month + 1 }
                } else if self.month <= 1 {
                    12
                } else {
                    self.month - 1
                };
                let dim = days_in_month(self.year, self.month);
                if self.day > dim {
                    self.day = dim;
                }
            }
            F_DAY => {
                let dim = days_in_month(self.year, self.month);
                self.day = if up {
                    if self.day >= dim { 1 } else { self.day + 1 }
                } else if self.day <= 1 {
                    dim
                } else {
                    self.day - 1
                };
            }
            F_HOUR => {
                self.hour = if up {
                    (self.hour + 1) % 24
                } else if self.hour == 0 {
                    23
                } else {
                    self.hour - 1
                };
            }
            F_MINUTE => {
                self.minute = if up {
                    (self.minute + 1) % 60
                } else if self.minute == 0 {
                    59
                } else {
                    self.minute - 1
                };
            }
            _ => {}
        }
    }
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
            if leap {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}

#[cfg(feature = "embassy-base")]
pub static DATE_PICKER: embassy_sync::blocking_mutex::Mutex<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    RefCell<Option<DatePicker>>,
> = embassy_sync::blocking_mutex::Mutex::new(RefCell::new(None));

#[cfg(feature = "simulator")]
pub static DATE_PICKER: std::sync::Mutex<RefCell<Option<DatePicker>>> =
    std::sync::Mutex::new(RefCell::new(None));

/// Open the picker.  `on_complete` runs on Fire/Execute; Cancel
/// dismisses without calling it.  All values are clamped to legal
/// ranges before storage.
pub fn begin(
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    on_complete: fn(u16, u8, u8, u8, u8),
    title: &'static str,
) {
    let picker = DatePicker::new(year, month, day, hour, minute, on_complete, title);
    #[cfg(feature = "embassy-base")]
    DATE_PICKER.lock(|cell| cell.replace(Some(picker)));
    #[cfg(feature = "simulator")]
    DATE_PICKER.lock().unwrap().replace(Some(picker));
}

pub fn dismiss() {
    #[cfg(feature = "embassy-base")]
    DATE_PICKER.lock(|cell| cell.replace(None));
    #[cfg(feature = "simulator")]
    DATE_PICKER.lock().unwrap().replace(None);
}

pub fn is_active() -> bool {
    #[cfg(feature = "embassy-base")]
    return DATE_PICKER.lock(|cell| cell.borrow().is_some());
    #[cfg(feature = "simulator")]
    return DATE_PICKER.lock().unwrap().borrow().is_some();
}

const LABELS: [&str; FIELD_COUNT as usize] = ["Year:", "Month:", "Day:", "Hour:", "Minute:"];

pub fn draw_overlay<D>(display: &mut D, picker: &DatePicker) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    // Clear background to white so the overlay stands alone.
    Rectangle::new(Point::new(0, 0), Size::new(152, 152))
        .into_styled(PrimitiveStyle::with_fill(WHITE))
        .draw(display)?;

    let centered = TextStyleBuilder::new()
        .baseline(Baseline::Middle)
        .alignment(Alignment::Center)
        .build();
    let left = TextStyleBuilder::new()
        .baseline(Baseline::Middle)
        .alignment(Alignment::Left)
        .build();

    // Title at top.
    Text::with_text_style(
        picker.title,
        Point::new(76, 14),
        MonoTextStyle::new(&FONT_7X13_BOLD, BLACK),
        centered,
    )
    .draw(display)?;

    for i in 0..FIELD_COUNT {
        let row_y = ROW_TOP_Y + i as i32 * ROW_H;
        let text_y = row_y + ROW_H / 2;
        let is_active = i == picker.field;

        if is_active {
            Rectangle::new(Point::new(0, row_y), Size::new(ROW_W, ROW_H as u32))
                .into_styled(PrimitiveStyle::with_fill(BLACK))
                .draw(display)?;
        }
        let fg = if is_active { WHITE } else { BLACK };

        // Label
        Text::with_text_style(
            LABELS[i as usize],
            Point::new(8, text_y),
            MonoTextStyle::new(&FONT_7X13_BOLD, fg),
            left,
        )
        .draw(display)?;

        // Value
        let mut buf: heapless::String<8> = heapless::String::new();
        match i {
            F_YEAR => {
                let _ = core::fmt::write(&mut buf, format_args!("{}", picker.year));
            }
            F_MONTH => {
                let _ = core::fmt::write(&mut buf, format_args!("{:02}", picker.month));
            }
            F_DAY => {
                let _ = core::fmt::write(&mut buf, format_args!("{:02}", picker.day));
            }
            F_HOUR => {
                let _ = core::fmt::write(&mut buf, format_args!("{:02}", picker.hour));
            }
            F_MINUTE => {
                let _ = core::fmt::write(&mut buf, format_args!("{:02}", picker.minute));
            }
            _ => {}
        }
        let value_right = TextStyleBuilder::new()
            .baseline(Baseline::Middle)
            .alignment(Alignment::Right)
            .build();
        Text::with_text_style(
            &buf,
            Point::new(144, text_y),
            MonoTextStyle::new(&FONT_7X13_BOLD, fg),
            value_right,
        )
        .draw(display)?;
    }

    // Hint at bottom.
    Text::with_text_style(
        "Up/Dn field  L/R value",
        Point::new(76, 132),
        MonoTextStyle::new(&FONT_6X10, BLACK),
        centered,
    )
    .draw(display)?;
    Text::with_text_style(
        "Fire=save  Cancel=drop",
        Point::new(76, 144),
        MonoTextStyle::new(&FONT_6X10, BLACK),
        centered,
    )
    .draw(display)?;

    Ok(())
}

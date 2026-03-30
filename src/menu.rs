use core::cell::RefCell;
use core::sync::atomic::Ordering;

use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_7X13},
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};

use crate::{BLACK, TriColor, WHITE};

// ── Item kinds ────────────────────────────────────────────────────────────────

pub enum MenuItemKind {
    Action(fn()),
    Submenu(&'static [MenuItem]),
    Back,
    /// Visual divider — not selectable; navigation skips over it.
    Separator,
}

pub struct MenuItem {
    pub label: fn() -> &'static str,
    pub kind: MenuItemKind,
}

// ── Per-screen navigation state ───────────────────────────────────────────────

/// Cursor position and optional active submenu for one screen.
pub struct ScreenState {
    root_items: &'static [MenuItem],
    root_pos: u8,
    /// `Some` while inside a submenu; `None` at the root level.
    sub_items: Option<&'static [MenuItem]>,
    sub_pos: u8,
}

impl ScreenState {
    pub const fn new(items: &'static [MenuItem]) -> Self {
        Self {
            root_items: items,
            root_pos: 0,
            sub_items: None,
            sub_pos: 0,
        }
    }

    pub fn current_items(&self) -> &'static [MenuItem] {
        match self.sub_items {
            Some(items) => items,
            None => self.root_items,
        }
    }

    pub fn current_pos(&self) -> usize {
        if self.sub_items.is_some() {
            self.sub_pos as usize
        } else {
            self.root_pos as usize
        }
    }

    fn current_pos_mut(&mut self) -> &mut u8 {
        if self.sub_items.is_some() {
            &mut self.sub_pos
        } else {
            &mut self.root_pos
        }
    }

    pub fn menu_up(&mut self) {
        let items = self.current_items();
        let pos = self.current_pos();
        if pos == 0 {
            return;
        }
        let mut prev = pos - 1;
        while prev > 0 && matches!(items[prev].kind, MenuItemKind::Separator) {
            prev -= 1;
        }
        if !matches!(items[prev].kind, MenuItemKind::Separator) {
            *self.current_pos_mut() = prev as u8;
        }
    }

    pub fn menu_down(&mut self) {
        let items = self.current_items();
        let len = items.len();
        let pos = self.current_pos();
        let mut next = pos + 1;
        while next < len && matches!(items[next].kind, MenuItemKind::Separator) {
            next += 1;
        }
        if next < len {
            *self.current_pos_mut() = next as u8;
        }
    }

    pub fn current_item(&self) -> &'static MenuItem {
        &self.current_items()[self.current_pos()]
    }

    /// Activate the currently selected item.
    ///
    /// - `Action` → call its function.
    /// - `Submenu` → push into the submenu, cursor reset to 0.
    /// - `Back` → pop back to the root menu.
    pub fn fire(&mut self) {
        match self.current_item().kind {
            MenuItemKind::Action(f) => f(),
            MenuItemKind::Submenu(items) => {
                self.sub_items = Some(items);
                self.sub_pos = 0;
            }
            MenuItemKind::Back => {
                self.sub_items = None;
            }
            MenuItemKind::Separator => {}
        }
    }

    pub fn get_label(&self, index: usize) -> Option<&'static str> {
        self.current_items().get(index).map(|item| (item.label)())
    }

    pub fn get_current_label(&self) -> Option<&'static str> {
        Some((self.current_item().label)())
    }
}

// ── Top-level display state ───────────────────────────────────────────────────

/// `M` screens, each with their own item list and cursor.
/// Left/right switches screens; up/down moves within the current screen.
pub struct DisplayState<const M: usize> {
    active_screen: u8,
    screens: [ScreenState; M],
}

#[allow(dead_code)]
impl<const M: usize> DisplayState<M> {
    pub const fn new(screens: [ScreenState; M]) -> Self {
        Self {
            active_screen: 0,
            screens,
        }
    }

    pub fn screen_left(&mut self) {
        if self.active_screen > 0 {
            self.active_screen -= 1;
        }
    }

    pub fn screen_right(&mut self) {
        if (self.active_screen as usize) + 1 < M {
            self.active_screen += 1;
        }
    }

    pub fn active_screen(&self) -> u8 {
        self.active_screen
    }

    pub fn current_screen(&self) -> &ScreenState {
        &self.screens[self.active_screen as usize]
    }

    pub fn current_screen_mut(&mut self) -> &mut ScreenState {
        &mut self.screens[self.active_screen as usize]
    }

    pub fn menu_up(&mut self) {
        self.current_screen_mut().menu_up();
    }

    pub fn menu_down(&mut self) {
        self.current_screen_mut().menu_down();
    }

    pub fn fire(&mut self) {
        self.current_screen_mut().fire();
    }

    pub fn get_current_menu_item(&self) -> Option<&'static str> {
        self.current_screen().get_current_label()
    }

    pub fn get_menu_item(&self, index: usize) -> Option<&'static str> {
        self.current_screen().get_label(index)
    }
}

// ── Action / label helpers ────────────────────────────────────────────────────

fn label_boost_rx() -> &'static str {
    if crate::BOOSTED_RX_GAIN.load(Ordering::Relaxed) {
        "Boost RX: ON"
    } else {
        "Boost RX: OFF"
    }
}

fn action_boost_rx() {
    let current = crate::BOOSTED_RX_GAIN.load(Ordering::Relaxed);
    crate::BOOSTED_RX_GAIN.store(!current, Ordering::Relaxed);
    #[cfg(feature = "embassy")]
    crate::BOOST_RX_CHANGED_SIGNAL.signal(());
}

fn action_reset_channels() {
    #[cfg(feature = "embassy")]
    crate::CHANNEL_RESET_SIGNAL.signal(());
}

fn action_reset_contacts() {
    #[cfg(feature = "embassy")]
    crate::CONTACT_RESET_SIGNAL.signal(());
}

fn action_melody_0() {
    #[cfg(feature = "embassy")]
    crate::fw::buzzer::play(0);
}

fn action_melody_1() {
    #[cfg(feature = "embassy")]
    crate::fw::buzzer::play(1);
}

fn action_melody_2() {
    #[cfg(feature = "embassy")]
    crate::fw::buzzer::play(2);
}

// ── Static item arrays ────────────────────────────────────────────────────────

static MELODY_ITEMS: [MenuItem; 4] = [
    MenuItem {
        label: || "< Back",
        kind: MenuItemKind::Back,
    },
    MenuItem {
        label: || "Startup",
        kind: MenuItemKind::Action(action_melody_0),
    },
    MenuItem {
        label: || "Rickroll",
        kind: MenuItemKind::Action(action_melody_1),
    },
    MenuItem {
        label: || "Imp. March",
        kind: MenuItemKind::Action(action_melody_2),
    },
];

static SETTINGS_ITEMS: [MenuItem; 5] = [
    MenuItem {
        label: || "< Back",
        kind: MenuItemKind::Back,
    },
    MenuItem {
        label: label_boost_rx,
        kind: MenuItemKind::Action(action_boost_rx),
    },
    MenuItem {
        label: || "",
        kind: MenuItemKind::Separator,
    },
    MenuItem {
        label: || "Reset channels",
        kind: MenuItemKind::Action(action_reset_channels),
    },
    MenuItem {
        label: || "Reset contacts",
        kind: MenuItemKind::Action(action_reset_contacts),
    },
];

static MAIN_ITEMS: [MenuItem; 4] = [
    MenuItem {
        label: || "Bornagotchi",
        kind: MenuItemKind::Action(|| {}),
    },
    MenuItem {
        label: || "Play melodies",
        kind: MenuItemKind::Submenu(&MELODY_ITEMS),
    },
    MenuItem {
        label: || "",
        kind: MenuItemKind::Separator,
    },
    MenuItem {
        label: || "Settings",
        kind: MenuItemKind::Submenu(&SETTINGS_ITEMS),
    },
];

static LORA_ITEMS: [MenuItem; 1] = [MenuItem {
    label: || "LoRa",
    kind: MenuItemKind::Action(|| {}),
}];

static ADVERT_ITEMS: [MenuItem; 1] = [MenuItem {
    label: || "Adverts",
    kind: MenuItemKind::Action(|| {}),
}];

static BADGERCORN_ITEMS: [MenuItem; 1] = [MenuItem {
    label: || "Badgercorn",
    kind: MenuItemKind::Action(|| {}),
}];

// ── DISPLAY_STATE ─────────────────────────────────────────────────────────────

#[cfg(feature = "embassy")]
use embassy_sync::blocking_mutex::{Mutex, raw::ThreadModeRawMutex};

#[cfg(feature = "embassy")]
pub static DISPLAY_STATE: Mutex<ThreadModeRawMutex, RefCell<DisplayState<4>>> =
    Mutex::new(RefCell::new(DisplayState::new([
        ScreenState::new(&MAIN_ITEMS),
        ScreenState::new(&LORA_ITEMS),
        ScreenState::new(&ADVERT_ITEMS),
        ScreenState::new(&BADGERCORN_ITEMS),
    ])));

#[cfg(feature = "simulator")]
use std::sync::Mutex;

#[cfg(feature = "simulator")]
pub static DISPLAY_STATE: Mutex<RefCell<DisplayState<4>>> =
    Mutex::new(RefCell::new(DisplayState::new([
        ScreenState::new(&MAIN_ITEMS),
        ScreenState::new(&LORA_ITEMS),
        ScreenState::new(&ADVERT_ITEMS),
        ScreenState::new(&BADGERCORN_ITEMS),
    ])));

// ── Scrolling menu renderer ───────────────────────────────────────────────────

/// Geometry constants for the 152×152 display.
///
/// The menu occupies y = 38..106, leaving room for the header (dots, battery,
/// device ID) above and the status banner below.
const MENU_X: i32 = 4;
const MENU_Y: i32 = 38;
const MENU_W: u32 = 144;
const ROW_H: i32 = 22;
const NUM_ROWS: usize = 3; // one above, center, one below

/// Draw a scrolling 3-item menu centered on `pos`.
///
/// - Center row: black background, white text (inverted).
/// - Adjacent rows (if items exist): black text on white background.
/// - A 1 px border frames the entire menu area.
/// - Submenu items have " >" appended to their label.
pub fn draw_menu<D>(display: &mut D, items: &[MenuItem], pos: usize) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    let text_style = TextStyleBuilder::new()
        .baseline(Baseline::Middle)
        .alignment(Alignment::Center)
        .build();
    let menu_h = ROW_H * NUM_ROWS as i32 + 2;

    // Outer border
    Rectangle::new(Point::new(MENU_X, MENU_Y), Size::new(MENU_W, menu_h as u32))
        .into_styled(PrimitiveStyle::with_stroke(BLACK, 1))
        .draw(display)?;

    for row in 0..NUM_ROWS {
        // item_idx: negative means "before the list" (no item to show).
        let item_idx = (pos as isize) + (row as isize) - 1;
        let row_y = MENU_Y + 1 + row as i32 * ROW_H;
        let text_y = row_y + ROW_H / 2;
        let is_center = row == 1;

        if is_center {
            Rectangle::new(
                Point::new(MENU_X + 1, row_y),
                Size::new(MENU_W - 2, ROW_H as u32),
            )
            .into_styled(PrimitiveStyle::with_fill(BLACK))
            .draw(display)?;
        }

        if item_idx >= 0 {
            if let Some(item) = items.get(item_idx as usize) {
                let fg = if is_center { WHITE } else { BLACK };
                if matches!(item.kind, MenuItemKind::Separator) {
                    // Draw a thin horizontal rule across the row
                    Rectangle::new(Point::new(MENU_X + 8, text_y), Size::new(MENU_W - 16, 1))
                        .into_styled(PrimitiveStyle::with_fill(fg))
                        .draw(display)?;
                } else {
                    let mut label: heapless::String<24> = heapless::String::new();
                    let _ = label.push_str((item.label)());
                    if matches!(item.kind, MenuItemKind::Submenu(_)) {
                        let _ = label.push_str(" >");
                    }
                    Text::with_text_style(
                        &label,
                        Point::new(MENU_X + MENU_W as i32 / 2, text_y),
                        MonoTextStyle::new(&FONT_7X13, fg),
                        text_style,
                    )
                    .draw(display)?;
                }
            }
        }
    }

    Ok(())
}

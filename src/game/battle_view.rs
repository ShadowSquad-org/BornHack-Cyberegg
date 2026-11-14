//! Battle screen — pick a friend from the mesh Friends list and fight.
//!
//! Opened from the top of the Play menu. Two internal states:
//! - **Picking**: scrollable friend list (mirrors `friends_view`), Up/Down
//!   moves the cursor, Fire challenges the highlighted friend, Cancel/any
//!   other button closes the screen.
//! - **Result**: a static report card (no live turn animation — e-paper
//!   refreshes are too slow for that) showing both pets' names, HP-left
//!   bars, and a WIN/LOSE banner. Any button returns to the game screen.

use crate::sync::SyncCell;
use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU16, AtomicU32, Ordering};

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::Text;

use super::battle::BattleOutcome;
use super::engine::PET_NAME_MAX;
use crate::menu::ButtonId;
use crate::ui::{self, TEXT_BLACK, TEXT_BOLD_BLACK, TEXT_BOLD_WHITE};
use crate::{BLACK, RED, TriColor, WHITE};

const BANNER_WON: &str = "YOU WON!";
const BANNER_LOST: &str = "YOU LOST";
const NO_FRIENDS_1: &str = "No friends";
const NO_FRIENDS_2: &str = "to battle yet";
const MSG_BUSY_TEXT: &str = "Can't battle now";
const MSG_NOT_READY_TEXT: &str = "Friend not ready";
const PICKER_HINT: &str = "Fire to challenge";
const RESULT_TITLE: &str = "Battle Result";
const NO_RESULT: &str = "No result yet";
const CLOSE_HINT: &str = "Any button to close";

/// Fixed texts of the Battle screens (host-only, for the guide's word list).
#[cfg(any(test, feature = "simulator"))]
pub fn words() -> [&'static str; 17] {
    [
        BANNER_WON,
        BANNER_LOST,
        NO_FRIENDS_1,
        NO_FRIENDS_2,
        MSG_BUSY_TEXT,
        MSG_NOT_READY_TEXT,
        PICKER_HINT,
        RESULT_TITLE,
        NO_RESULT,
        CLOSE_HINT,
        "Battle <w>W-<l>L",
        "On cooldown <n>s",
        "+<n> HAX",
        "You",
        "Friend",
        "HP",
        "<n>/<m>",
    ]
}

const STATE_PICKING: u8 = 0;
const STATE_RESULT: u8 = 1;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static STATE: AtomicU8 = AtomicU8::new(STATE_PICKING);
static CURSOR: AtomicU8 = AtomicU8::new(0);

/// Transient picker feedback: shown when a challenge can't proceed, so a
/// Fire press always tells the player *why* nothing happened instead of
/// silently no-op'ing. Cleared on cursor move / (re)open / a real battle.
const MSG_NONE: u8 = 0;
const MSG_COOLDOWN: u8 = 1; // Battle on cooldown; remaining secs in PICKER_MSG_SECS.
const MSG_BUSY: u8 = 2; // Pet asleep or mid-action.
const MSG_NOT_READY: u8 = 3; // Friend hasn't broadcast combat stats yet.
static PICKER_MSG: AtomicU8 = AtomicU8::new(MSG_NONE);
static PICKER_MSG_SECS: AtomicU16 = AtomicU16::new(0);

fn set_picker_msg(msg: u8, secs: u16) {
    PICKER_MSG.store(msg, Ordering::Relaxed);
    PICKER_MSG_SECS.store(secs, Ordering::Relaxed);
}

fn clear_picker_msg() {
    PICKER_MSG.store(MSG_NONE, Ordering::Relaxed);
}

/// Result of the most recently resolved battle, stashed for the Result
/// screen to render. `None` until a battle has actually been fought.
struct ResultDisplay {
    friend_name: [u8; PET_NAME_MAX],
    friend_name_len: u8,
    outcome: BattleOutcome,
    /// The HAX the win paid (0 when none).
    hax: u32,
}

static RESULT: SyncCell<Option<ResultDisplay>> = SyncCell::new(None);

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

/// The result screen (HP bars + WON/LOST + reward) needs a full tri-color
/// refresh — the red BANNER_LOST banner under-drives on the fast delta LUT,
/// and the whole-screen picker→result change ghosts otherwise.
pub fn wants_full_refresh() -> bool {
    ACTIVE.load(Ordering::Relaxed) && STATE.load(Ordering::Relaxed) == STATE_RESULT
}

/// Test hook: back to the idle, closed state.
#[cfg(test)]
pub(crate) fn reset_for_test() {
    ACTIVE.store(false, Ordering::Relaxed);
    STATE.store(STATE_PICKING, Ordering::Relaxed);
}

/// Test hook: show the result card as a finished battle would.
#[cfg(test)]
pub(crate) fn show_result_for_test() {
    ACTIVE.store(true, Ordering::Relaxed);
    STATE.store(STATE_RESULT, Ordering::Relaxed);
}

/// When the result screen was shown, so the same button action that
/// resolved the battle (or a joystick bounce right after) can't instantly
/// dismiss it before the player sees it.
static RESULT_SHOWN_MS: AtomicU32 = AtomicU32::new(0);
const RESULT_DISMISS_GUARD_MS: u32 = 600;

// ── Battle animation ─────────────────────────────────────────────────────────
//
// Full-screen two-stage result (see the battle-animation spec). Own pet on the
// left (art as-is), opponent on the right (same art mirrored in software). The
// takeover flag + timing live in `game::mod` (`battle_anim_*`); this module
// only renders a frame for the current stage.

use super::BattleStage;
use super::engine::anim_files::{BATTLE_AA_LOST, BATTLE_AA_STAND, BATTLE_AA_WON, battle_filename};

/// Left pet origin (own pet).
const ANIM_LEFT_X: i32 = 4;
/// Right pet origin (opponent, mirrored).
const ANIM_RIGHT_X: i32 = 84;
/// Top of both pet sprites.
const ANIM_PET_Y: i32 = 34;

/// Pick the pose animation code for one combatant. Stage 1 is always standing;
/// stage 2 gives the viewer's own pet the won/lost pose and the opponent the
/// opposite. `is_own` selects which side; `viewer_won` is from the viewer's
/// perspective.
fn pose_aa(stage: BattleStage, viewer_won: bool, is_own: bool) -> u8 {
    match stage {
        BattleStage::Standing => BATTLE_AA_STAND,
        BattleStage::Result => {
            let won = if is_own { viewer_won } else { !viewer_won };
            if won { BATTLE_AA_WON } else { BATTLE_AA_LOST }
        }
        // Not drawn — the takeover ends at Done.
        BattleStage::Done => BATTLE_AA_STAND,
    }
}

/// Draw the stage-2 win/lose banner (red, centred near the bottom).
fn draw_anim_banner<D>(display: &mut D, viewer_won: bool) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    let text = if viewer_won { BANNER_WON } else { BANNER_LOST };
    let ts = crate::ui::CENTERED;
    // White strip behind the banner so it reads over either pet.
    Rectangle::new(Point::new(0, 122), Size::new(152, 22))
        .into_styled(PrimitiveStyle::with_fill(WHITE))
        .draw(display)?;
    let style = crate::ui::TEXT_BOLD_RED;
    Text::with_text_style(text, Point::new(76, 133), style, ts).draw(display)?;
    Ok(())
}

/// Sprite edge length of the battle poses (`72x72` PCX).
const ANIM_PET_SIZE: i32 = 72;
/// Name-label baseline, in the band between the pets' feet (y=106) and the
/// stage-2 banner strip (y=122).
const ANIM_NAME_Y: i32 = 108;
/// Horizontal centre of each fighter's sprite column. The right sprite runs
/// past the 152px panel edge, so its label centres on the *visible* part.
const ANIM_LEFT_CX: i32 = ANIM_LEFT_X + ANIM_PET_SIZE / 2;
const ANIM_RIGHT_CX: i32 = (ANIM_RIGHT_X + 152) / 2;

/// Longest name that fits a 72px sprite column at 6px/char.
const ANIM_NAME_MAX: usize = 12;

/// Label both fighters under their sprites: the player's pet name on the
/// left, the opponent's on the right. Falls back to the species name when a
/// pet is unnamed, or when the opponent never beaconed us (so they aren't in
/// the friends list) — better than a blank label. Drawn in both stages.
fn draw_anim_names<D>(display: &mut D, own_kind: u8, opp_kind: u8) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    use embedded_graphics::mono_font::MonoTextStyle;
    use embedded_graphics::mono_font::iso_8859_1::FONT_6X10;

    let ts = crate::ui::CENTERED_TOP;
    let style = MonoTextStyle::new(&FONT_6X10, BLACK);

    let own = super::lifecycle::pet_name();
    let own = if own.is_empty() {
        super::engine::PetKind::from_u8(own_kind).name()
    } else {
        own
    };
    crate::ui::draw_text(
        display,
        crate::truncate_str(own, ANIM_NAME_MAX),
        Point::new(ANIM_LEFT_CX, ANIM_NAME_Y),
        style,
        ts,
    )?;

    let opp_buf;
    let opp = match super::friends::name_of(super::battle_anim_opp_id()) {
        Some((buf, len)) => {
            opp_buf = buf;
            super::engine::name_str(&opp_buf, len)
        }
        None => "",
    };
    let opp = if opp.is_empty() {
        super::engine::PetKind::from_u8(opp_kind).name()
    } else {
        opp
    };
    crate::ui::draw_text(
        display,
        crate::truncate_str(opp, ANIM_NAME_MAX),
        Point::new(ANIM_RIGHT_CX, ANIM_NAME_Y),
        style,
        ts,
    )?;

    Ok(())
}

/// Power-bar geometry (Street-Fighter style: top-left own, top-right opponent).
const HP_BAR_Y: i32 = 4;
const HP_BAR_H: u32 = 8;
const HP_BAR_W: u32 = 66;
const HP_LEFT_X: i32 = 4;
const HP_RIGHT_X: i32 = 82;

/// Draw one power bar: a black outline filled proportionally to `pct`. The own
/// bar (left) fills from the left edge; the opponent bar (right, `anchor_right`)
/// depletes toward the centre, mirroring the fighting-game convention.
fn draw_hp_bar<D>(display: &mut D, x: i32, pct: u8, anchor_right: bool) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    Rectangle::new(Point::new(x, HP_BAR_Y), Size::new(HP_BAR_W, HP_BAR_H))
        .into_styled(PrimitiveStyle::with_stroke(BLACK, 1))
        .draw(display)?;
    let fw = HP_BAR_W * pct.min(100) as u32 / 100;
    if fw > 0 {
        let fx = if anchor_right {
            x + (HP_BAR_W - fw) as i32
        } else {
            x
        };
        Rectangle::new(Point::new(fx, HP_BAR_Y), Size::new(fw, HP_BAR_H))
            .into_styled(PrimitiveStyle::with_fill(BLACK))
            .draw(display)?;
    }
    Ok(())
}

/// Draw both power bars for `stage`: 100/100 while standing, the outcome HP
/// once the result stage begins.
fn draw_hp_bars<D>(display: &mut D, stage: BattleStage) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    let (own_hp, opp_hp) = if stage == BattleStage::Standing {
        (100, 100)
    } else {
        super::battle_anim_hp()
    };
    draw_hp_bar(display, HP_LEFT_X, own_hp, false)?;
    draw_hp_bar(display, HP_RIGHT_X, opp_hp, true)?;
    Ok(())
}

/// Firmware render of the current battle-animation frame. Blits both pets from
/// FAT12 (own left, opponent right-mirrored) and the stage-2 banner. The caller
/// clears the display and drives the refresh.
#[cfg(feature = "embassy-core")]
pub async fn render_anim<D>(display: &mut D, stage: BattleStage)
where
    D: embedded_graphics::draw_target::DrawTarget<Color = TriColor>
        + crate::epd_driver::PlaneAccess,
{
    use crate::fw::fat12;

    let (own_kind, opp_kind, viewer_won) = super::battle_anim_ctx();

    let own_name = battle_filename(own_kind, pose_aa(stage, viewer_won, true));
    if let Ok(file) = fat12::find_file(&own_name).await {
        super::sprite_loader::blit_file(display, &file, ANIM_LEFT_X, ANIM_PET_Y, false).await;
    }
    let opp_name = battle_filename(opp_kind, pose_aa(stage, viewer_won, false));
    if let Ok(file) = fat12::find_file(&opp_name).await {
        super::sprite_loader::blit_file(display, &file, ANIM_RIGHT_X, ANIM_PET_Y, true).await;
    }
    let _ = draw_hp_bars(display, stage);
    let _ = draw_anim_names(display, own_kind, opp_kind);
    if stage == BattleStage::Result {
        let _ = draw_anim_banner(display, viewer_won);
    }
}

/// Simulator render of the current battle-animation frame (host build).
#[cfg(all(feature = "simulator", not(feature = "embassy-core")))]
pub fn draw_anim_sim<D>(display: &mut D)
where
    D: DrawTarget<Color = TriColor>,
{
    let (own_kind, opp_kind, viewer_won) = super::battle_anim_ctx();
    let stage = super::battle_anim_stage();

    let own_name = battle_filename(own_kind, pose_aa(stage, viewer_won, true));
    super::sprite_loader::blit_pcx_sim(display, &own_name, ANIM_LEFT_X, ANIM_PET_Y, false);
    let opp_name = battle_filename(opp_kind, pose_aa(stage, viewer_won, false));
    super::sprite_loader::blit_pcx_sim(display, &opp_name, ANIM_RIGHT_X, ANIM_PET_Y, true);
    let _ = draw_hp_bars(display, stage);
    let _ = draw_anim_names(display, own_kind, opp_kind);
    if stage == BattleStage::Result {
        let _ = draw_anim_banner(display, viewer_won);
    }
}

pub fn open() {
    STATE.store(STATE_PICKING, Ordering::Relaxed);
    CURSOR.store(0, Ordering::Relaxed);
    clear_picker_msg();
    ACTIVE.store(true, Ordering::Relaxed);
}

pub fn close() {
    ACTIVE.store(false, Ordering::Relaxed);
}

/// Challenge the highlighted friend, if any. No-op (stays on the picker)
/// if there are no friends yet, no pet is active, or Battle is still on
/// cooldown. That last check is normally already enforced by the Play
/// menu (a disabled `Item::Battle` can't be activated to get here at
/// all), but re-checking it here means this screen can never trigger a
/// second challenge while already open, regardless of how it was
/// reached.
fn try_challenge() {
    let Some(stats) = super::lifecycle::cycle() else {
        return;
    };
    if !stats.can_battle {
        // Give feedback instead of silently ignoring the tap.
        if stats.cooldown_battle > 0 {
            // cooldown is in ticks (1 tick = 10 s).
            set_picker_msg(MSG_COOLDOWN, stats.cooldown_battle.saturating_mul(10));
        } else {
            set_picker_msg(MSG_BUSY, 0);
        }
        return;
    }

    let idx = CURSOR.load(Ordering::Relaxed) as usize;
    let Some(friend) = super::friends::get(idx) else {
        return;
    };
    let Some((outcome, hax)) = super::battle::challenge(&friend) else {
        // Friend hasn't broadcast combat stats yet (or no local stats).
        set_picker_msg(MSG_NOT_READY, 0);
        return;
    };

    clear_picker_msg();
    unsafe {
        *RESULT.get() = Some(ResultDisplay {
            friend_name: friend.name,
            friend_name_len: friend.name_len,
            outcome,
            hax,
        });
    }
    RESULT_SHOWN_MS.store(crate::platform::uptime_ms_u32(), Ordering::Relaxed);
    STATE.store(STATE_RESULT, Ordering::Relaxed);
}

/// Route a button press while the Battle screen is active. Owns its own
/// input logic across the two sub-states (mirrors `pet_select`).
pub fn handle_input(btn: ButtonId) {
    if STATE.load(Ordering::Relaxed) == STATE_RESULT {
        // Any button dismisses the result — but not for a brief guard
        // window after it appears, so the Fire that resolved the battle
        // (or a joystick bounce) can't flick it away before it's seen.
        let elapsed =
            crate::platform::uptime_ms_u32().wrapping_sub(RESULT_SHOWN_MS.load(Ordering::Relaxed));
        if elapsed >= RESULT_DISMISS_GUARD_MS {
            close();
        }
        return;
    }

    match btn {
        ButtonId::Up => {
            clear_picker_msg();
            super::friends::cursor_prev(&CURSOR);
        }
        ButtonId::Down => {
            clear_picker_msg();
            super::friends::cursor_next(&CURSOR);
        }
        // Both the joystick Fire and the Execute button confirm — players
        // reach for either, and only accepting Fire made Execute silently
        // close the screen ("no result").
        ButtonId::Fire | ButtonId::Execute => try_challenge(),
        _ => close(),
    }
}

pub fn draw<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    crate::ui::clear_screen(display)?;

    if STATE.load(Ordering::Relaxed) == STATE_RESULT {
        draw_result(display)
    } else {
        draw_picker(display)
    }
}

/// Picker title carrying the lifetime record, e.g. `Battle 3W-1L`.
pub fn picker_title(wins: u16, losses: u16) -> heapless::String<21> {
    use core::fmt::Write;
    let mut title = heapless::String::new();
    let _ = write!(title, "Battle {}W-{}L", wins, losses);
    title
}

fn draw_picker<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    let (wins, losses) = super::lifecycle::battle_record().unwrap_or((0, 0));
    ui::draw_title_bar(display, &picker_title(wins, losses), Point::zero(), 152)?;

    let count = super::friends::count();
    if count == 0 {
        // Two lines — the single string is wider than the 152px panel and
        // got clipped to "o friends to battle ye".
        ui::draw_centered_message(display, NO_FRIENDS_1, Point::new(76, 78))?;
        ui::draw_centered_message(display, NO_FRIENDS_2, Point::new(76, 92))?;
        return Ok(());
    }

    let cursor = CURSOR.load(Ordering::Relaxed) as usize;
    let left = crate::ui::TOP_LEFT;
    let row_h = 24i32;
    let start_y = 22;

    // Fixed 4-row page window; the visible page follows the cursor.
    const PAGE: usize = 4;
    let viewport_start = (cursor / PAGE) * PAGE;
    let visible = PAGE.min(count as usize - viewport_start);

    for i in 0..visible {
        let idx = viewport_start + i;
        let Some(friend) = super::friends::get(idx) else {
            break;
        };
        let is_selected = idx == cursor;
        let y = start_y + i as i32 * row_h;

        if is_selected {
            Rectangle::new(Point::new(2, y - 1), Size::new(148, row_h as u32 - 2))
                .into_styled(PrimitiveStyle::with_fill(BLACK))
                .draw(display)?;
        }

        let line = friend.label();
        let style = if is_selected {
            TEXT_BOLD_WHITE
        } else {
            TEXT_BOLD_BLACK
        };
        crate::ui::draw_text(display, line.as_str(), Point::new(6, y + 3), style, left)?;
    }

    // Feedback line when a previous Fire couldn't start a battle.
    match PICKER_MSG.load(Ordering::Relaxed) {
        MSG_COOLDOWN => {
            let secs = PICKER_MSG_SECS.load(Ordering::Relaxed);
            let mut m: heapless::String<24> = heapless::String::new();
            let _ = core::fmt::Write::write_fmt(&mut m, format_args!("On cooldown {}s", secs));
            ui::draw_centered_message(display, m.as_str(), Point::new(76, 132))?;
        }
        MSG_BUSY => ui::draw_centered_message(display, MSG_BUSY_TEXT, Point::new(76, 132))?,
        MSG_NOT_READY => {
            ui::draw_centered_message(display, MSG_NOT_READY_TEXT, Point::new(76, 132))?
        }
        _ => {}
    }

    let hint = crate::ui::CENTERED_BOTTOM;
    Text::with_text_style(PICKER_HINT, Point::new(76, 150), TEXT_BLACK, hint)
        .draw(display)?;

    Ok(())
}

fn draw_result<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    ui::draw_title_bar(display, RESULT_TITLE, Point::zero(), 152)?;

    let result = unsafe { &*RESULT.get() };
    let Some(result) = result else {
        ui::draw_centered_message(display, NO_RESULT, Point::new(76, 85))?;
        return Ok(());
    };

    let left = crate::ui::TOP_LEFT;
    let friend_name = super::engine::name_str(&result.friend_name, result.friend_name_len);
    let friend_label = if friend_name.is_empty() {
        "Friend"
    } else {
        friend_name
    };

    Text::with_text_style("You", Point::new(6, 26), TEXT_BOLD_BLACK, left).draw(display)?;
    super::stat_bar::draw_stat_bar(
        display,
        "HP",
        result.outcome.challenger_hp_pct,
        Point::new(6, 42),
        Point::new(30, 40),
        Size::new(116, 16),
        if result.outcome.challenger_hp_pct == 0 {
            RED
        } else {
            BLACK
        },
    )?;

    crate::ui::draw_text(
        display,
        friend_label,
        Point::new(6, 66),
        TEXT_BOLD_BLACK,
        left,
    )?;
    super::stat_bar::draw_stat_bar(
        display,
        "HP",
        result.outcome.target_hp_pct,
        Point::new(6, 82),
        Point::new(30, 80),
        Size::new(116, 16),
        if result.outcome.target_hp_pct == 0 {
            RED
        } else {
            BLACK
        },
    )?;

    let banner = if result.outcome.challenger_won {
        BANNER_WON
    } else {
        BANNER_LOST
    };
    let centered = crate::ui::CENTERED;
    let banner_color = if result.outcome.challenger_won {
        TEXT_BOLD_BLACK
    } else {
        crate::ui::TEXT_BOLD_RED
    };
    Text::with_text_style(banner, Point::new(76, 112), banner_color, centered).draw(display)?;

    // Show the HAX the win paid.
    if result.hax > 0 {
        let mut reward: heapless::String<16> = heapless::String::new();
        let _ = core::fmt::Write::write_fmt(
            &mut reward,
            format_args!("+{} HAX", result.hax),
        );
        Text::with_text_style(
            reward.as_str(),
            Point::new(76, 130),
            TEXT_BOLD_BLACK,
            centered,
        )
        .draw(display)?;
    }

    let hint = crate::ui::CENTERED_BOTTOM;
    Text::with_text_style(CLOSE_HINT, Point::new(76, 148), TEXT_BLACK, hint)
        .draw(display)?;

    Ok(())
}

#[cfg(test)]
mod anim_tests {
    use super::*;

    #[test]
    fn pose_selection() {
        // Stage 1: both standing regardless of outcome/side.
        assert_eq!(pose_aa(BattleStage::Standing, true, true), BATTLE_AA_STAND);
        assert_eq!(
            pose_aa(BattleStage::Standing, false, false),
            BATTLE_AA_STAND
        );
        // Stage 2, viewer won: own pet WON, opponent LOST.
        assert_eq!(pose_aa(BattleStage::Result, true, true), BATTLE_AA_WON);
        assert_eq!(pose_aa(BattleStage::Result, true, false), BATTLE_AA_LOST);
        // Stage 2, viewer lost: own pet LOST, opponent WON.
        assert_eq!(pose_aa(BattleStage::Result, false, true), BATTLE_AA_LOST);
        assert_eq!(pose_aa(BattleStage::Result, false, false), BATTLE_AA_WON);
    }

    #[test]
    fn name_labels_fit_the_sprite_column() {
        // Short names pass through untouched.
        assert_eq!(crate::truncate_str("Rex", ANIM_NAME_MAX), "Rex");
        // 12 chars is exactly the 72px column at 6px/char — the longest
        // species name ("Bartholomeus") must survive intact.
        assert_eq!(
            crate::truncate_str("Bartholomeus", ANIM_NAME_MAX),
            "Bartholomeus"
        );
        assert_eq!(crate::truncate_str("Bartholomeus", ANIM_NAME_MAX).len(), 12);
        // Longer names are clipped rather than overflowing the panel.
        assert_eq!(
            crate::truncate_str("AAAAAAAAAAAAAAAA", ANIM_NAME_MAX),
            "AAAAAAAAAAAA"
        );
        // Multi-byte codepoints are never split mid-sequence.
        let s = crate::truncate_str("ÆÆÆÆÆÆÆÆ", ANIM_NAME_MAX); // 8 x 2 bytes = 16
        assert!(s.len() <= 12);
        assert_eq!(s, "ÆÆÆÆÆÆ");
    }

    #[test]
    fn name_labels_clear_the_banner_and_sprites() {
        // Labels sit below the pets' feet and above the banner strip, so
        // neither the sprite nor the WON/LOST banner overlaps them.
        assert!(ANIM_NAME_Y >= ANIM_PET_Y + ANIM_PET_SIZE);
        assert!(ANIM_NAME_Y + 10 <= 122);
        // Both labels centre inside the 152px panel.
        assert!(ANIM_LEFT_CX > 0 && ANIM_LEFT_CX < 152);
        assert!(ANIM_RIGHT_CX > 0 && ANIM_RIGHT_CX < 152);
    }
}

#[cfg(test)]
mod title_tests {
    use super::picker_title;

    #[test]
    fn the_battle_picker_shows_your_record() {
        assert_eq!(picker_title(3, 1).as_str(), "Battle 3W-1L");
        assert!(picker_title(u16::MAX, u16::MAX).len() <= 21);
    }
}

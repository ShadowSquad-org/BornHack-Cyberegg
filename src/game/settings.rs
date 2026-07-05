//! BornPets settings persisted to the KV store.
//!
//! Currently just the difficulty preset selected via the on-badge menu.
//! Picking a new mode writes here; it takes effect on the next reboot
//! (live-switching would let an in-flight pet see different decay rates
//! between ticks).

use core::sync::atomic::{AtomicU8, Ordering};

#[cfg(feature = "embassy-base")]
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
#[cfg(feature = "embassy-base")]
use embassy_sync::signal::Signal;

use super::engine::thresholds::Mode;

const KV_NAMESPACE: &str = "game";
const KV_KEY_MODE: &str = "mode";

/// The mode the user has currently selected in the menu.  Mirrors the
/// on-disk value; differs from `thresholds::active_mode()` only between
/// the user toggling the menu setting and the next reboot.
static PENDING_MODE: AtomicU8 = AtomicU8::new(Mode::DEFAULT as u8);

#[cfg(feature = "embassy-base")]
static SETTINGS_DIRTY: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// Load the persisted mode from KV.  Returns [`Mode::DEFAULT`] (Classic)
/// on first boot or if the stored value is malformed.
#[cfg(feature = "embassy-base")]
pub async fn load_mode_from_kv() -> Mode {
    let ns = crate::fw::kv::namespace(KV_NAMESPACE);
    let mut buf = [0u8; 1];
    let stored = match ns.get(KV_KEY_MODE, &mut buf).await {
        Ok(1) => Mode::from_u8(buf[0]),
        _ => Mode::DEFAULT,
    };
    PENDING_MODE.store(stored as u8, Ordering::Relaxed);
    stored
}

/// Record the menu-selected mode and ask the persister task to flush
/// it to KV.  Sync — safe to call from menu action handlers.
pub fn request_mode_change(mode: Mode) {
    PENDING_MODE.store(mode as u8, Ordering::Relaxed);
    #[cfg(feature = "embassy-base")]
    SETTINGS_DIRTY.signal(());
}

/// Mode currently selected in the menu (NOT necessarily the one the
/// engine is running with — that's `thresholds::active_mode()`).
pub fn pending_mode() -> Mode {
    Mode::from_u8(PENDING_MODE.load(Ordering::Relaxed))
}

/// `true` when the on-disk / menu choice differs from the value the
/// engine actually loaded at boot — the user changed mode after boot
/// and a reboot is needed for it to take effect.
pub fn pending_differs_from_active() -> bool {
    pending_mode() as u8 != super::engine::thresholds::active_mode() as u8
}

/// Drains [`SETTINGS_DIRTY`] and persists the pending mode to KV.
/// Spawn once at boot.
#[cfg(feature = "embassy-base")]
#[embassy_executor::task]
pub async fn persister_task() {
    let ns = crate::fw::kv::namespace(KV_NAMESPACE);
    loop {
        SETTINGS_DIRTY.wait().await;
        let mode = pending_mode();
        let _ = ns.set(KV_KEY_MODE, &[mode as u8], true).await;
        defmt::info!("game settings: persisted mode={}", mode.label());
    }
}

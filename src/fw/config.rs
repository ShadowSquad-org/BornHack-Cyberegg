//! Generic device-config flags persisted to the `"config"` kv namespace.
//!
//! Currently small (one flag — boot chime).  Add new flags here when
//! they don't have an obvious home in an existing namespace
//! (`"watch"` for alarm/clock state, `"meshcore"` for radio/identity,
//! etc.).
//!
//! Each flag follows the same shape used elsewhere in the firmware:
//!
//!   * a `pub static` `AtomicBool` (or atomic of choice) in `crate` that the
//!     rest of the firmware reads synchronously;
//!   * a `pub static` `Signal<()>` fired when the value changes; and
//!   * a small async task that waits on the signal and writes the current
//!     atomic value to flash.
//!
//! The boot path loads the persisted value into the atomic once at
//! startup, then spawns the persister task.  Menu actions only need
//! to mutate the atomic and fire the signal — they don't need to
//! be async themselves.

use crate::fw::kv;

fn ns() -> kv::KvNamespace {
    kv::namespace("config")
}

// ── Boot chime ──────────────────────────────────────────────────────────────

/// Read the persisted boot-chime flag.  Defaults to `true` (chime
/// enabled) on a fresh badge that's never had the user toggle this
/// setting — first boot still gets the audible "I'm ready" signal.
pub async fn get_boot_chime() -> bool {
    let mut b = [0u8; 1];
    !matches!(ns().get("boot_chime", &mut b).await, Ok(1) if b[0] == 0)
}

/// Persist the boot-chime flag to flash.
pub async fn set_boot_chime(enabled: bool) -> Result<(), kv::KvError> {
    ns().set("boot_chime", &[enabled as u8], true).await
}

/// Wait on `BOOT_CHIME_CHANGED_SIGNAL` and persist the current
/// `BOOT_CHIME_ENABLED` value to flash whenever it fires.  Spawn
/// once at boot, after `kv::init()`.
#[embassy_executor::task]
pub async fn boot_chime_persister_task() {
    use core::sync::atomic::Ordering;
    loop {
        crate::BOOT_CHIME_CHANGED_SIGNAL.wait().await;
        let v = crate::BOOT_CHIME_ENABLED.load(Ordering::Relaxed);
        match set_boot_chime(v).await {
            Ok(()) => defmt::debug!("config: boot_chime={=bool} persisted", v),
            Err(e) => defmt::warn!("config: boot_chime persist failed: {:?}", e),
        }
    }
}

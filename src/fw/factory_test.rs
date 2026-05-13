//! Factory test gate — runs on first boot, stamps a KV flag on pass,
//! skips automatically on subsequent boots.
//!
//! Phase 1 (current): scaffolding only.  A 2 s LED-driven splash, then
//! unconditionally stamps the KV pass flag.  The integration plumbing
//! (KV key, namespace, [`is_passed`] / [`run`] entry points) is the
//! point — the real test logic lands in later phases.
//!
//! ## Planned phases
//!
//! 1. **Skeleton + KV gate** (this file) — `run()` auto-passes after a
//!    short splash.  Lets the rest of the firmware exercise the gate
//!    without depending on real test logic landing first.
//! 2. **Automatic tests** — port from `src/bin/hwtest.rs`: HFXO, LFXO,
//!    QSPI flash JEDEC + write-read, SX1262 LoRa version, SSD1675 EPD
//!    BUSY transitions, battery ADC sanity, buzzer.
//! 3. **Interactive tests** — joystick + buttons, LED cycle, qwiic
//!    continuity, display test pattern with human sign-off.
//! 4. **Polish** — beep codes for audible feedback, retry-from-fail
//!    path, debug override (e.g. button combo at boot to clear the
//!    KV flag and re-run).
//!
//! ## Recovery (developer override)
//!
//! No combo wired up yet.  Phase 4 will add a "Cancel + Execute held
//! at boot" override that clears [`KEY_PASSED`] and re-enters the
//! test path.  Until then, clearing the badge via the bootloader's
//! factory-reset combo (Execute + Cancel + Fire on power-up) wipes
//! the whole KV store and forces a re-test.

use crate::fw::kv;
use embassy_time::Timer;

/// KV namespace used by the factory-test gate.  Kept separate from
/// settings / game / mesh so a future "factory diagnostics" submenu
/// can dump its own state without poking other namespaces.
const NAMESPACE: &str = "hwtest";

/// Sentinel key — presence-only check via [`KvNamespace::exists`].
/// Stored value is a 4-byte LE marker (currently `0x01`) reserved for
/// a future "schema version" if we ever need to retest after firmware
/// rev bumps.
const KEY_PASSED: &str = "passed";

/// Returns `true` if the badge has already passed the factory test
/// (i.e. the [`KEY_PASSED`] flag is present in the `"hwtest"` KV
/// namespace).  Errors are treated as "not passed" so a flaky read
/// at boot doesn't silently let an untested badge through.
pub async fn is_passed() -> bool {
    matches!(kv::namespace(NAMESPACE).exists(KEY_PASSED).await, Ok(true))
}

/// Stamp the KV flag.  Logs but does not propagate write errors —
/// a failed write just means the test re-runs on next boot, which
/// is the safer fallback for a one-shot factory-floor gate.
pub async fn mark_passed() {
    let store = kv::namespace(NAMESPACE);
    match store.set(KEY_PASSED, &1u32.to_le_bytes(), true).await {
        Ok(()) => defmt::info!("hwtest: marked passed"),
        Err(e) => defmt::warn!("hwtest: failed to mark passed: {:?}", e),
    }
}

/// Phase 1 stub: 2 s splash then auto-mark passed.
///
/// Future phases replace the body with real per-peripheral checks
/// and only call [`mark_passed`] when every check returns `Ok`.
/// On a fail, the function will instead loop displaying results +
/// beeping the matching `ERR_*` code — see [`crate::fw::factory_test`]
/// module docs for the phase plan.
pub async fn run() {
    defmt::info!("hwtest: factory test entered (Phase 1 stub)");
    Timer::after_millis(2000).await;
    mark_passed().await;
    defmt::info!("hwtest: stub complete — continuing to normal boot");
}

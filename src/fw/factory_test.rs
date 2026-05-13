//! Factory test gate — runs on first boot, stamps a KV flag on pass,
//! skips automatically on subsequent boots.
//!
//! ## Phase 2 (current)
//!
//! Two raw-register peripheral checks:
//!
//! - **HFXO** (32 MHz crystal): verify `HFCLKSTAT.STATE` reports the
//!   crystal is the current source.
//! - **LFXO** (32.768 kHz crystal): verify `LFCLKSTAT` reports it
//!   running, with source = Xtal.
//!
//! Both are non-invasive: just register reads, no peripheral
//! ownership.  Triggered late enough in boot that `embassy_nrf::init`
//! has already requested HFXO on healthy badges, so checking
//! `STATE = 1` confirms the request actually completed.
//!
//! **Caveat for cold-boot HFXO failure**: if HFXO never starts, the
//! `HfclkSource::ExternalXtal` config in `bin/embassy.rs::main`
//! makes `embassy_nrf::init()` itself block forever waiting for the
//! `HFCLKSTARTED` event, so we never reach this gate.  Detecting
//! that failure requires a separate pre-init register check (Phase
//! 3+).  For now the cold-boot HFXO case is handled by the
//! `hfxo-workaround` branch which switches the config to `Internal`.
//!
//! ## Planned phases
//!
//! 1. Skeleton + KV gate ✓
//! 2. Automatic post-init clock checks (this commit) — HFXO + LFXO
//!    state reads.  Lays the [`TestResults`] infrastructure for
//!    future tests to plug into.
//! 3. Peripheral-ownership tests — QSPI JEDEC re-read, SX1262 LoRa
//!    version, SSD1675 EPD BUSY transitions, battery ADC sanity,
//!    buzzer.  Requires `bin/embassy.rs::main` to thread peripheral
//!    handles into [`run`].
//! 4. Interactive — joystick + buttons, LED cycle, qwiic continuity,
//!    full-screen display test pattern with human sign-off.
//! 5. Polish — beep codes, retry-from-fail path, dev override.
//!
//! ## Failure behaviour
//!
//! On any test failure the gate enters an infinite loop without
//! stamping the KV flag.  This:
//!
//! - Keeps the badge in factory-test mode across reboots until the
//!   issue is fixed (re-flash / re-flow), so a factory worker can
//!   spot the dead badge.
//! - Lets a future Phase 3 LED/display indicator surface *which*
//!   test failed.
//! - Does not call [`mark_passed`] — the badge boots back into the
//!   test next power-up.
//!
//! ## Recovery (developer override)
//!
//! No combo wired up yet.  Phase 4 will add a "Cancel + Execute
//! held at boot" override that clears [`KEY_PASSED`].  Until then,
//! the bootloader's factory-reset combo (Execute + Cancel + Fire on
//! power-up) wipes the KV store and forces a re-test.

use crate::fw::kv;
use embassy_time::Timer;

// ---------------------------------------------------------------------------
// KV gate
// ---------------------------------------------------------------------------

/// KV namespace used by the factory-test gate.  Kept separate from
/// settings / game / mesh so a future "factory diagnostics" submenu
/// can dump its own state without poking other namespaces.
const NAMESPACE: &str = "hwtest";

/// Sentinel key — presence-only check via [`crate::fw::kv::KvNamespace::exists`].
/// Stored value is a 4-byte LE marker (currently `0x01`) reserved for
/// a future "schema version" if we ever need to retest after firmware
/// rev bumps.
const KEY_PASSED: &str = "passed";

/// Returns `true` if the badge has already passed the factory test.
/// Errors are treated as "not passed" so a flaky read at boot doesn't
/// silently let an untested badge through.
pub async fn is_passed() -> bool {
    matches!(kv::namespace(NAMESPACE).exists(KEY_PASSED).await, Ok(true))
}

/// Stamp the KV flag.  Logs but does not propagate write errors —
/// a failed write just means the test re-runs on next boot, which
/// is the safer fallback for a one-shot factory-floor gate.
pub async fn mark_passed() {
    match kv::namespace(NAMESPACE)
        .set(KEY_PASSED, &1u32.to_le_bytes(), true)
        .await
    {
        Ok(()) => defmt::info!("hwtest: marked passed"),
        Err(e) => defmt::warn!("hwtest: failed to mark passed: {:?}", e),
    }
}

// ---------------------------------------------------------------------------
// Result accounting
// ---------------------------------------------------------------------------

/// Stable per-test error code.  Matches `bin/hwtest.rs::ERR_*` so beep-code
/// lookups against `HWTEST.md` agree across both binaries.
#[derive(Copy, Clone, Eq, PartialEq, defmt::Format)]
pub enum TestCode {
    Hfxo,
    Lfxo,
}

impl TestCode {
    /// Matches the numeric `ERR_*` constants in `bin/hwtest.rs`.
    pub const fn beep_code(self) -> u8 {
        match self {
            Self::Hfxo => 22,
            Self::Lfxo => 21,
        }
    }
}

/// Fixed-capacity accumulator for failed checks.  Each variant in
/// [`TestCode`] can fail at most once per run, so a small heapless
/// `Vec` (or array slot) is enough; using a `u32` bit-mask keeps the
/// type Copy and avoids dragging `heapless` in for two slots.
#[derive(Default, Copy, Clone)]
pub struct TestResults {
    fail_mask: u32,
}

impl TestResults {
    pub fn record(&mut self, code: TestCode, ok: bool) {
        if !ok {
            self.fail_mask |= 1 << code.beep_code();
        }
    }

    pub fn any_failed(&self) -> bool {
        self.fail_mask != 0
    }
}

// ---------------------------------------------------------------------------
// nRF52840 CLOCK peripheral — raw register addresses (PAC-free).
// ---------------------------------------------------------------------------
//
// Matches `bin/hwtest.rs`.  Read-only here in Phase 2; Phase 3 may
// add the request/timeout pattern used by `probe_hfxo` / `probe_lfxo`
// in the standalone test binary.

const CLOCK_HFCLKSTAT: *const u32 = 0x4000_0408 as *const u32;
const CLOCK_LFCLKSTAT: *const u32 = 0x4000_0418 as *const u32;

/// `HFCLKSTAT.STATE` bit (16): 1 when HFXO is the current source.
const HFCLKSTAT_STATE_RUNNING: u32 = 1 << 16;
/// `LFCLKSTAT.STATE` bit (16): 1 when LFCLK is running.
const LFCLKSTAT_STATE_RUNNING: u32 = 1 << 16;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// HFXO check — confirm the 32 MHz crystal is the current HFCLK
/// source post-`embassy_nrf::init`.  On a healthy badge with
/// `HfclkSource::ExternalXtal`, `STATE = 1` by the time we get here
/// because the init waited for the start event.  If init *had*
/// hung on a broken crystal we wouldn't be running — that case
/// needs the pre-init detection path (Phase 3+).
fn check_hfxo() -> bool {
    let stat = unsafe { CLOCK_HFCLKSTAT.read_volatile() };
    let running = stat & HFCLKSTAT_STATE_RUNNING != 0;
    if running {
        defmt::info!("hwtest: HFXO check passed (HFCLKSTAT={:#010x})", stat);
    } else {
        defmt::warn!(
            "hwtest: HFXO check FAILED — HFCLKSTAT={:#010x}, expected STATE bit set",
            stat,
        );
    }
    running
}

/// LFXO check — `embassy_nrf::init` always brings LFCLK up (RTC1 is
/// LFCLK-driven), so this should always pass on any working chip
/// regardless of which low-frequency source is in use.
fn check_lfxo() -> bool {
    let stat = unsafe { CLOCK_LFCLKSTAT.read_volatile() };
    let running = stat & LFCLKSTAT_STATE_RUNNING != 0;
    if running {
        defmt::info!("hwtest: LFXO check passed (LFCLKSTAT={:#010x})", stat);
    } else {
        defmt::warn!(
            "hwtest: LFXO check FAILED — LFCLKSTAT={:#010x}, expected STATE bit set",
            stat,
        );
    }
    running
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run the Phase 2 test suite.  Returns normally on pass (after
/// stamping the KV flag); loops forever on fail so the badge stays
/// in factory-test mode until repaired or KV-cleared.
pub async fn run() {
    defmt::info!("hwtest: factory test entered (Phase 2)");

    // Small splash so any future LED/EPD indicator has time to
    // register visually before tests start firing.
    Timer::after_millis(500).await;

    let mut results = TestResults::default();
    results.record(TestCode::Hfxo, check_hfxo());
    results.record(TestCode::Lfxo, check_lfxo());

    if results.any_failed() {
        defmt::error!(
            "hwtest: one or more checks FAILED (fail_mask={:#x}) — \
             holding in factory-test mode, KV flag NOT stamped",
            results.fail_mask,
        );
        // Spin forever, feeding nothing.  The shared watchdog task
        // is already running so the badge will reset on its own,
        // hit this gate again, fail again, and stay visibly broken
        // until a technician intervenes.
        loop {
            Timer::after_millis(1000).await;
        }
    }

    mark_passed().await;
    defmt::info!("hwtest: all checks passed — continuing to normal boot");
}

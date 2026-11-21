//! Read `EPD.CFG` from the USB-MSC FAT12 partition into an
//! [`ssd1680::EpdTuning`] (SSD1680 builds only).
//!
//! Same format as `BPETS_V3.CFG`: one `KEY=VALUE` per line, `#` starts a
//! comment, whitespace around `=` ignored, unknown keys logged and skipped.
//! Numbers are decimal or `0x` hex, switches `1/0/on/off`. Every value is
//! clamped into `ssd1680::config::limits` by the driver crate, so no file
//! content can select a harmful VCOM or frame count. No file = defaults.
//!
//! ```text
//! # try a stronger VCOM and a shorter preview
//! VCOM=0x50
//! PREVIEW_FRAMES=15
//! ```
//!
//! Keys (defaults): `PREVIEW=1`, `PREVIEW_FRAMES=20`, `PREVIEW_MAX_PX=4000`,
//! `PREVIEW_MAX_RUN=8`, `SETTLE_MS=1500`, `SHORT_DELTA=0`, `VCOM=0x40`,
//! `LEDGER=0`, `LEDGER_POS=72`, `LEDGER_NEG=65`, `LEDGER_DECAY=3`,
//! `CORR_FRAMES=1`, `HALO_FRAMES=4`. `LEDGER` only gates the correction
//! pulses (the charge booking always runs); set `LEDGER=1` to try them.
//! `HALO_FRAMES` is the halo kick after a full drive: white pixels next to
//! freshly driven black get that many frames (5 ms each) of VSL to pull back
//! the black ink bleeding into them; `0` = off, else clamped to 1..20. It runs
//! whether `LEDGER` is on or not (each kick is still booked in the ledger and
//! skipped for pixels already at `-LEDGER_NEG`).

use ssd1680::{EpdTuning, KvResult};

/// The tuning file; the defmt literals below must match (defmt needs literals).
const CFG_FILE: &str = "EPD.CFG";
const READ_CHUNK_LEN: usize = 256;
/// Longest line kept whole; longer lines are dropped, never half-applied.
const LINE_BUF_LEN: usize = 96;

/// Apply one line to `t`; logs clamps and unknown keys.
fn apply_line(t: &mut EpdTuning, line: &[u8]) {
    let Some((key, value)) = crate::cfg_text::kv_of_line(line, true) else {
        return;
    };
    if key.is_empty() || value.is_empty() {
        return;
    }
    let name = core::str::from_utf8(key).unwrap_or("?");
    match t.apply_kv(key, value) {
        KvResult::Applied => defmt::info!("EPD.CFG: {=str} set", name),
        KvResult::Clamped => defmt::warn!("EPD.CFG: {=str} out of range, clamped", name),
        KvResult::BadValue => defmt::warn!("EPD.CFG: {=str} bad value, ignored", name),
        KvResult::UnknownKey => defmt::warn!("EPD.CFG: unknown key {=str}, ignored", name),
    }
}

/// Load `EPD.CFG` on top of [`EpdTuning::default`]. Call after
/// `fat12::format_if_needed()`. A missing file or a read error leaves the
/// defaults (a read error keeps the lines already applied, minus a cut-off
/// last line).
pub async fn load() -> EpdTuning {
    use crate::cfg_text::LineAssembler;
    use crate::fw::fat12;

    let mut t = EpdTuning::default();
    let file = match fat12::to_8_3(CFG_FILE) {
        Some(name) => fat12::find_file(&name).await.ok(),
        None => None,
    };
    let Some(file) = file else {
        defmt::info!("EPD.CFG: not found, defaults");
        return t;
    };

    let mut lines = LineAssembler::<LINE_BUF_LEN>::new();
    let mut chunk = [0u8; READ_CHUNK_LEN];
    let mut offset = 0u32;
    let mut read_failed = false;
    loop {
        let n = match fat12::read_file(&file, offset, &mut chunk).await {
            Ok(n) => n,
            Err(_) => {
                read_failed = true;
                0
            }
        };
        if n == 0 {
            break;
        }
        offset = offset.saturating_add(n as u32);
        lines.push(&chunk[..n], &mut |l| apply_line(&mut t, l));
    }
    if !read_failed {
        lines.finish(&mut |l| apply_line(&mut t, l));
    }
    let t = t.clamped();
    defmt::info!(
        "EPD.CFG: preview={} frames={} max_px={} run={} settle_ms={} short={} vcom={=u8:#x} ledger={} pos={} neg={} decay={} corr={} halo={}",
        t.preview,
        t.preview_frames,
        t.preview_max_px,
        t.preview_max_run,
        t.settle_ms,
        t.short_delta,
        t.vcom,
        t.ledger,
        t.ledger_pos,
        t.ledger_neg,
        t.ledger_decay,
        t.corr_frames,
        t.halo_frames
    );
    t
}

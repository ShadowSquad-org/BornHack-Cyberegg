# Custom e-paper LUT (`LUT.CFG`)

The badge normally drives its e-paper panel with the waveform LUT it
reads from the panel's own OTP at boot. You can override that with a
**calibrated waveform** — e.g. one exported from the `ssd1675-calibration`
tool — by dropping a `LUT.CFG` file onto the badge's USB drive. No
reflash needed.

## How to use

1. Plug the badge in via USB — it appears as a small removable drive
   ("FAT12 Storage").
2. Copy `LUT.CFG` (see format below) to the root of that drive.
3. Eject and reset the badge. On boot it loads the LUT and drives the
   panel with it.

To go back to the panel's built-in waveform, delete `LUT.CFG` (or hold
**Fire** at boot — see recovery).

## File format

Plain text, `KEY=VALUE` per line, `#` starts a comment — the same style
as `PETS.CFG` / `BORNPETS.CFG`. Only two keys are read; anything else is
ignored, so you can trim a calibration-tool export down by hand.

```
# CyberAegg EPD custom LUT
variant=A                 # A = SSD1675 / SSD1675A, B = SSD1675B (required)
band_lut=08992144...      # 214 hex chars = one 107-byte LUT unit
speed=100                 # optional: LUT cycle-duration scale (0..255, 100 = OEM)
```

- **`variant`** — `A` or `B`. **Must match your panel.** The badge
  auto-detects its own panel variant and *ignores the file* on a
  mismatch: an A-panel LUT on a B panel (or vice-versa) uses the wrong
  row layout and drive voltages and can blank or stress the display.
- **`band_lut`** — the 107-byte register-0x33 LUT unit as hex (exactly
  214 hex chars). This is the `band_lut` field from the calibration
  tool's JSON export; the trailer timing/voltage bytes are already baked
  into it. Applied to **all** temperature bands as a base.
- **`band_lut_00` … `band_lut_15`** *(optional)* — override a single
  temperature band (0 = coldest … 15 = warmest). Supply a full set for a
  **temperature-compensated** custom LUT, or just a few to tweak specific
  bands. Any band you don't set (via `band_lut` or `band_lut_NN`) keeps
  the panel's OTP-probed waveform for that temperature.
- **`speed`** *(optional)* — scales every non-zero LUT timing byte before
  each refresh (`100` = OEM duration, lower = faster/lighter, higher =
  slower/darker). Same knob as the on-device menu, but bundled with the
  waveform; a value here wins over the menu/persisted value at boot.

The multi-stage `stage_luts` and the staged-drive `controls` from the
calibration tool's full export are **not** used by this path — the badge
firmware runs the single-LUT refresh engine.

### Size

No practical limit — the file is streamed off flash in chunks, so a
full 16-band temperature-compensated export (~3.7 KB, base plus all 16
`band_lut_NN` overrides, comments and all) loads fine. The only hard
constraint is per *line*: a single line longer than ~2.8 KB is rejected,
and no legitimate LUT line comes anywhere near that (~230 bytes).

### `speed` floor

`speed` is clamped to **30..255** everywhere it can be set (file, menu,
persisted value). Values below 30 behave as 30 — that is the fastest /
lightest refresh allowed.

### If the badge boots fine but ignores your LUT

Rejection is silent (log-only). Checklist:

1. **Wrong `variant`** — the panel is auto-detected; a mismatched file
   is skipped. Try the other letter.
2. **Wrong key** — the calibration tool's full JSON export calls things
   `stage_luts` / `controls`; the badge wants the flat `band_lut` hex
   field. Copy that one.
3. **Bad hex / wrong length** — each LUT value must be exactly 214 hex
   chars; any parse error anywhere rejects the whole file.
4. **Fire held at boot** — forces OTP for that boot (see recovery).

## Recovery — if a LUT renders badly

Hold **Fire** (the joystick centre press) while the badge boots. This
forces the safe OTP waveform and ignores `LUT.CFG` for that boot, so you
can always get a readable screen back even if a custom LUT blanked it.
Delete or fix the file, then reboot normally.

The badge also rejects a `LUT.CFG` that is malformed, the wrong length,
or the wrong variant, falling back to OTP automatically.

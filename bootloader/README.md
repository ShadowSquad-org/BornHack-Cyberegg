# CyberAegg Bootloader

A minimal USB DFU bootloader for the nRF52840-based CyberAegg badge.
It occupies the first 64 KB of internal flash, leaving ~960 KB for the
application — roughly twice the space available under the old embassy-boot-nrf
setup.

---

## How it works

On every power-on or reset the bootloader runs first and takes one of three
paths, decided by which buttons are held at the moment the chip comes out of
reset:

| Buttons held at boot      | Action                                        |
| ------------------------- | --------------------------------------------- |
| None                      | Validate app vector table → jump to app       |
| Execute only              | USB DFU mode — receive new firmware over USB  |
| Execute + Cancel + Fire   | Factory reset — erase QSPI flash, then reset  |

### Normal boot

The bootloader reads the first two words of the app's vector table
(initial SP and reset vector) and checks that they are plausible
Cortex-M values:

- SP must be within the 256 KB RAM region (`0x20000000–0x20040000`)
- Reset vector must be an odd (Thumb) address within the app flash region

If the check passes the bootloader sets VTOR, loads the app's SP, and
branches to its reset handler. If it fails it logs a warning and halts,
waiting for the user to enter DFU mode and flash a valid image.

### USB DFU mode

The bootloader exposes a standard USB DFU 1.1 interface (class `0xFE`,
subclass `0x01`, protocol `0x02`). The host tool `dfu-util` can flash a
raw binary directly to the app partition.

The red LED blinks slowly while waiting for a connection, turns solid blue
during the download, and blinks green three times before the device resets
into the new application.

To enter DFU mode:

1. Hold the **Execute** button.
2. Power-cycle the board (or press reset while keeping Execute held).
3. Release Execute — the red LED should start blinking.
4. Flash from the host:

```sh
make dfu-flash          # debug build
make dfu-flash-release  # release build
```

`make dfu-flash` builds the app, converts the ELF to a raw binary, and
calls `dfu-util -w -D <binary>`. The `-w` flag makes dfu-util wait up to
10 seconds for the device to appear, so you can run the command before the
board has fully enumerated.

You can also call dfu-util directly if you already have a binary:

```sh
dfu-util -w -D firmware.bin
```

### Factory reset *(requires `with-qspi-flash` feature)*

Holding **Execute + Cancel + Fire** at boot erases the entire QSPI flash
chip (the 2 MB ZD25WQ16C that holds the key-value store and bond data).
The red LED blinks while the erase is in progress (~40 seconds worst case)
and the device resets automatically when done. Internal flash (firmware) is
**not** affected — the app continues to run normally after the reset, but
all stored settings, bonds, and channels are cleared.

---

## Flash partition layout

```text
0x00000000 – 0x0000FFFF   Bootloader    (64 KB)
0x00010000 – 0x000FFFFF   Application   (960 KB)
```

The boundary is defined in two places that must be kept in sync:

| File                  | Symbol / origin                                    |
| --------------------- | -------------------------------------------------- |
| `bootloader/memory.x` | `FLASH LENGTH = 64K` and `APP_START = 0x00010000`  |
| `memory.x` (app)      | `FLASH ORIGIN = 0x00010000, LENGTH = 960K`         |

---

## Building and flashing the bootloader

The bootloader is a standalone Cargo project separate from the main
workspace. All commands below should be run from inside the `bootloader/`
directory, or via the Makefile targets in the project root.

### Build

```sh
# from project root
make bl

# or directly
cd bootloader && cargo build --release
```

### First-time installation (via SWD)

A full chip erase is required before programming so that any stale content
from a previous bootloader does not interfere:

```sh
make bl-flash
```

This runs `probe-rs erase --chip nRF52840_xxAA` followed by
`probe-rs download`. After programming, flash the app once via SWD to give
the bootloader a valid image to boot:

```sh
make flash          # SWD, debug build
make flash-release  # SWD, release build
```

Subsequent app updates can be done over USB DFU without a debugger.

### Monitor bootloader RTT output

```sh
make bl-monitor
```

---

## Adapting to a different board

The bootloader has several board-specific concerns.

### 1. Button pins

In `src/main.rs`, the boot-mode decision reads GPIO inputs:

```rust
let btn_exe = Input::new(board!(p, btn_exe), Pull::Up);
```

`board!` is a macro defined in `src/board.rs` (which `include!`s the
main app's `src/fw/board.rs`). Replace these with whatever pins your board
uses, or read the GPIO registers directly:

```rust
// Example: read P0.26 directly without the board macro
use embassy_nrf::gpio::{Input, Pull};
let dfu_pin = Input::new(p.P0_26, Pull::Up);
let dfu_requested = dfu_pin.is_low();
drop(dfu_pin);
```

### 2. LED pins

The LED pins used for status feedback are passed into `dfu::dfu_task` and
`dfu::factory_reset_and_reset`. On the CyberAegg they are active-low RGB
LEDs. Swap them for whatever indicator your board has, or remove the LED
code entirely if none is available.

### 3. QSPI flash (factory reset only)

Factory reset is gated behind the `with-qspi-flash` Cargo feature. If your
board has no external QSPI flash, simply omit the feature (it is not in the
default set) and the factory-reset path is compiled out entirely.

If your board does have QSPI flash but on different pins, enable
`with-qspi-flash` and update the pin arguments in `dfu::factory_reset_and_reset`
and its call site in `main.rs`.

### 4. Flash partition layout

If you need a larger or smaller bootloader, change the `LENGTH` in
`bootloader/memory.x` and update `APP_START` to match. Then update
`FLASH ORIGIN` in the app's `memory.x` to the same address.

Example — 32 KB bootloader:

```text
# bootloader/memory.x
FLASH (rx)  : ORIGIN = 0x00000000, LENGTH = 32K
APP_START = 0x00008000;

# memory.x  (app)
FLASH (rx)  : ORIGIN = 0x00008000, LENGTH = 992K
```

### 5. RAM size

The nRF52840 has 256 KB of RAM. If you port to a chip with a different RAM
size, update `RAM LENGTH` in `bootloader/memory.x` **and** the SP validity
range in `main.rs`:

```rust
// Adjust upper bound to ORIGIN(RAM) + LENGTH(RAM)
let sp_ok = (0x2000_0000..=0x2004_0000).contains(&sp);
```

---

## Source layout

```text
bootloader/
├── memory.x          Flash/RAM layout — defines APP_START
├── Cargo.toml        Dependencies (embassy-nrf, embassy-usb, …)
├── .cargo/
│   └── config.toml   Build target, probe-rs runner, DEFMT_LOG level
└── src/
    ├── board.rs      Re-exports the main app's board pin macro
    ├── nvmc.rs       Raw NVMC register writes (erase_page, write)
    ├── dfu.rs        DFU 1.1 handler, USB task, factory reset
    └── main.rs       App validation, jump, entry point, fault handlers
```

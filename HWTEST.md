# Cyber Ægg Hardware Test Firmware

`hwtest` is a standalone firmware image for verifying a freshly assembled
Cyber Ægg PCB before the production firmware is loaded. It runs without a
bootloader, boots straight from reset, performs a fixed sequence of
low-level checks, and signals the result on the RGB LED and the buzzer.

## Intended use

Flash `hwtest` at the point where an assembled PCB has just come off the
line. The workflow is:

1. Power the board (battery or test harness, SWD debugger attached).
1. `make flash-hwtest` — erases the chip, programs `hwtest`, releases
   reset. Total time: ~3 seconds.
1. Observe the LED and buzzer:
   - **Green + short ascending chime** → all checks passed.
   - **Red + repeating beep sequence** → one or more checks failed.
     Decode the beeps (see below) to identify the faulty circuits,
     repair, re-test.

`hwtest` is *not* meant to ship with the device. It is a factory-only
diagnostic image.

## Prerequisites

- SWD programmer supported by `probe-rs` (the J-Link and DAPLink-class
  debuggers work).
- `probe-rs` installed (`cargo install probe-rs --features cli`).
- `arm-none-eabi-size` (optional, used by the Makefile targets to print a
  size summary after each build).

The firmware must be downloaded to the device over SWD. Without a
bootloader, the nRF52840's USB port is inactive, so USB flashing is
not available.

The badge also needs to be powered while the test runs. If the SWD
programmer does not supply power to the target, connect a USB-C cable
to the badge to power it externally.

The EPD panel does **not** need to be fitted during testing — `hwtest`
treats the EPD signal lines as bare GPIOs. A populated panel is
tolerated, see *Beep codes* below for the BUSY caveat.

## Build and flash targets

| Command               | Behaviour                                                                                               |
| --------------------- | ------------------------------------------------------------------------------------------------------- |
| `make fw-hwtest`      | Build only. Prints the text/data/bss size summary.                                                      |
| `make flash-hwtest`   | Erase chip, program `hwtest`, release reset. For production use — exits as soon as programming is done. |
| `make run-hwtest`     | Erase, program, then attach the RTT console. Use during development to see the defmt log stream.        |
| `make monitor-hwtest` | Attach RTT to an already-running hwtest (no flash, no reset).                                           |

Under the hood `make fw-hwtest` runs:

```
cargo build --bin hwtest --no-default-features \
            --features hwtest --target thumbv7em-none-eabihf \
            --profile release-hwtest
```

The `release-hwtest` profile inherits from `release` but keeps symbols and
full DWARF so `probe-rs` can decode defmt messages and emit source
locations. The loaded flash sections are identical to a stripped release
build.

## Memory layout

`hwtest` uses a dedicated `memory-hwtest.x` that places the image at
`0x0000_0000` — **no bootloader required**.

## LED feedback

| State                | Meaning                                                                                                                              |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| White (all three on) | `hwtest` is running, checks in progress. Typically visible only at boot — the test sequence completes in a few hundred milliseconds. |
| Solid green          | All checks passed.                                                                                                                   |
| Solid red            | One or more checks failed. The buzzer communicates which.                                                                            |

## Buzzer encoding

Failure codes are emitted in a tally-mark encoding:

- Every code = `(code / 5)` long beeps + `(code % 5)` short beeps.
- Long beep: 600 ms.
- Short beep: 150 ms.
- 150 ms gap between beeps within one code.
- 800 ms gap between different codes.
- 2 s pause before the whole sequence loops.

Examples:

| Code | Beeps                                  |
| ---- | -------------------------------------- |
| 1    | short                                  |
| 4    | short short short short                |
| 5    | long                                   |
| 9    | long short short short short           |
| 10   | long long                              |
| 14   | long long short short short short      |
| 19   | long long long short short short short |
| 20   | long long long long                    |

## Beep code reference

Legend: **—** = long beep (600 ms), **•** = short beep (150 ms). Read
left-to-right, long beeps first, then short beeps.

| Code | Pattern                                | Spoken                                 | Check                |
| ---: | :------------------------------------- | :------------------------------------- | :------------------- |
|    1 | `•`                                    | short                                  | Cancel button        |
|    2 | `• •`                                  | short short                            | Execute button       |
|    3 | `• • •`                                | short short short                      | Joystick Up          |
|    4 | `• • • •`                              | short short short short                | Joystick Down        |
|    5 | `—`                                    | long                                   | Joystick Left        |
|    6 | `— •`                                  | long short                             | Joystick Right       |
|    7 | `— • •`                                | long short short                       | Joystick Fire        |
|    8 | `— • • •`                              | long short short short                 | LoRa SX1262          |
|    9 | `— • • • •`                            | long short short short short           | Battery voltage      |
|   10 | `— —`                                  | long long                              | QSPI flash JEDEC ID  |
|   11 | `— — •`                                | long long short                        | QWIIC SDA            |
|   12 | `— — • •`                              | long long short short                  | QWIIC SCL            |
|   13 | *(never emitted — informational only)* | —                                      | EPD BUSY             |
|   14 | `— — • • • •`                          | long long short short short short      | EPD RESET            |
|   15 | `— — —`                                | long long long                         | EPD DC               |
|   16 | `— — — •`                              | long long long short                   | EPD CSN              |
|   17 | `— — — • •`                            | long long long short short             | EPD SCK              |
|   18 | `— — — • • •`                          | long long long short short short       | EPD MOSI             |
|   19 | `— — — • • • •`                        | long long long short short short short | PS_SYNC              |
|   20 | `— — — —`                              | long long long long                    | Buzzer pin pull-down |
|   21 | `— — — — •`                            | long long long long short              | 32.768 kHz LFXO start |
|   22 | `— — — — • •`                          | long long long long short short        | 32 MHz HFXO start |

## Check list / beep codes

| Code | Test                                                                                                                                                          |
| ---: | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1 | Cancel button pulled high (internal pull-up)                                                                                                                  |
|    2 | Execute button pulled high                                                                                                                                    |
|    3 | Joystick Up pulled high + not shorted to another joystick pin                                                                                                 |
|    4 | Joystick Down pulled high + not shorted                                                                                                                       |
|    5 | Joystick Left pulled high + not shorted                                                                                                                       |
|    6 | Joystick Right pulled high + not shorted                                                                                                                      |
|    7 | Joystick Fire pulled high + not shorted                                                                                                                       |
|    8 | LoRa SX1262 responds to a `GetStatus` SPI command after reset                                                                                                 |
|    9 | Battery voltage is inside [3000 mV, 4400 mV] (SAADC via the on-board divider)                                                                                 |
|   10 | External QSPI flash returns a plausible JEDEC ID (neither all-0x00 nor all-0xFF)                                                                              |
|   11 | QWIIC SDA pulled high by the external bus pull-up + not shorted to SCL                                                                                        |
|   12 | QWIIC SCL pulled high + not shorted to SDA                                                                                                                    |
|   13 | *(informational only)* EPD BUSY low at rest or low during shorts scan. A populated EPD panel drives BUSY itself, so this is logged but never fails the board. |
|   14 | EPD RESET — not shorted to any other EPD line, pulled high with internal pull-up                                                                              |
|   15 | EPD DC — same                                                                                                                                                 |
|   16 | EPD CSN — same                                                                                                                                                |
|   17 | EPD SCK — same                                                                                                                                                |
|   18 | EPD MOSI — same                                                                                                                                               |
|   19 | PS_SYNC driven high by the power supply circuit (no internal pull; genuinely needs an external drive)                                                         |
|   20 | Buzzer pin idles low through the 1 MΩ PCB pull-down                                                                                                           |
|   21 | 32.768 kHz LFXO starts within 1 s of `TASKS_LFCLKSTART`; detects dry joints or a missing/damaged crystal that would otherwise only surface as bad BLE timing   |
|   22 | 32 MHz HFXO starts within 1 s of `TASKS_HFCLKSTART`; the HFXO is required by the radio peripherals (LoRa, BLE). On a missing/bad crystal the chip still runs on HFINT, so the CPU boots and GPIO/SPI tests can pass while the radio would fail later |

### Short detection

For groups with multiple pins (joystick, QWIIC, EPD), each pin in turn is
driven low as an output while the others are sampled as inputs. If any
neighbour reads low while the first is driven, *both* codes are reported
— so a short between Joystick Up (3) and Joystick Down (4) lights codes 3
**and** 4, which appear as two separate beep groups in the repeating
sequence.

## defmt log stream

Every step, every reading, and every fault is printed over RTT with
source locations enabled. Use `make run-hwtest` during development — the
RTT console is attached before reset is released, so the boot banner and
every check are captured in order.

Sample output (all-pass board):

```
0.000000 [INFO] hwtest: starting
0.001x   [INFO] hwtest: checking buttons pulled high
0.02x    [INFO] hwtest: checking joystick pulled high
0.04x    [INFO] hwtest: checking QWIIC bus pulled high
0.06x    [INFO] hwtest: checking EPD lines pulled high
0.08x    [INFO] hwtest: joystick short-to-neighbour scan
0.1xx    [INFO] hwtest: QWIIC short-to-neighbour scan
0.1xx    [INFO] hwtest: EPD short-to-neighbour scan
0.2xx    [INFO] hwtest: LoRa GetStatus = 0x2A
0.2xx    [INFO] hwtest: vbat = 3920 mV
0.2xx    [INFO] hwtest: JEDEC ID: EF 40 18
0.3xx    [INFO] hwtest: PASS
```

Log level is controlled by the `DEFMT_LOG` env var in `.cargo/config.toml`
(default `info`).

## Interpreting the result

- **Green LED, pass chime** → board is good.
- **Red LED, any beep sequence** → decode each beep group, look up the
  code, repair that area of the PCB, re-test. If multiple codes sound
  together and share a pair of adjacent pins, suspect a solder bridge
  rather than two independent faults.
- **No LED at all** → the CPU never reached `main`. Check the power
  rails, the HFXO crystal, and SWD connectivity. `make run-hwtest` and
  look for stack-trace output.
- **Repeating failure that doesn't match any real fault** → verify the
  test harness: the joystick must be at rest, QWIIC connector empty
  (unless you intend to pull the lines low), battery must be within
  range, and the EPD cable either fully seated or not fitted at all.

## Known symptoms and fixes

A running list of observed hardware faults, how they surface (either in
`hwtest` beep codes or in the running main firmware), and what to do
about them on the bench.  Add new entries here as they are found — one
row per distinct symptom + fix pair.

| Symptom                                                                                                                                           | How it surfaces                                                                                                                                                                                                                 | Likely cause                                                                                                                            | Fix                                                                                                                                                            |
| ------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| BLE device is discoverable but pairing fails part-way through (SMP timeout, `PairingFailed` in the defmt log, or the phone shows a generic error); **or** the badge pairs but connections drop / reconnect intermittently and notifications come through erratically | `hwtest` beeps code 21 (LFXO start).  Main firmware: advertising still works (uses the HFXO started by `embassy_nrf::init`) while pairing and the connection interval depend on the 32.768 kHz LFXO for accurate timing. | LFXO (32 kHz) crystal has a bad solder joint or a dry pad.  The crystal either doesn't start, starts slowly, or drifts, so the radio's connection-event timing falls outside the negotiated window. | Reflow / resolder the 32.768 kHz crystal and its two load caps.                                                                                                |
| One or more buttons always read low, or never change state                                                                                        | `hwtest` beeps codes 1–2 (Cancel / Execute) or 3–7 (joystick)                                                                                                                                                                   | Cold joints on the nRF52840 pins driving the affected GPIO lines (see `board.rs` pin map).                                              | Reflow / resolder the nRF52840 package.  Inspect the relevant corner of the BGA/QFN with a microscope.                                                         |
| EPD refuses to respond, shows garbage, or `hwtest` reports EPD signal failures                                                                    | `hwtest` beeps codes 14–18 (EPD RESET / DC / CSN / SCK / MOSI shorts or stuck lines).  Code 13 (BUSY) is informational only — a real fault here usually surfaces on one of the other five.                                      | Cold joints on the nRF52840 pins connected to the EPD connector.                                                                        | Reflow / resolder the nRF52840.  If the BUSY line is the only one misbehaving with the panel fitted, also inspect the EPD FPC connector.                      |
| LoRa radio never answers on SPI or the chip's 32 MHz TCXO does not start                                                                          | `hwtest` beeps code 8 (LoRa SX1262).  Main firmware logs `lora init failed` or `set_rx failed`.                                                                                                                                 | Cold joints on the nRF52840 pins driving SPI2 (SCK/MOSI/MISO/NSS) or on the SX1262 itself, or on the SX1262's TCXO/oscillator network.  | Reflow / resolder the nRF52840, then the SX1262 and its oscillator network if the fault persists.                                                              |
| LoRa never transmits / receives, or BLE connection parameters drift enough for the phone to give up, but CPU and GPIO peripherals work fine      | `hwtest` beeps code 22 (HFXO start).  Main firmware's radio layer may appear "on" but never makes contact with peers.                                                                                                            | 32 MHz HFXO crystal has a bad joint / load cap, or the crystal itself is damaged.  HFCLK falls back to HFINT (±5 % accuracy) which is not enough for radio timing. | Reflow / resolder the 32 MHz crystal and its two load caps.                                                                                                    |

### Notes

- Many of the "resolder the MCU" entries collapse into the same
  underlying cause: a single cold joint on the nRF52840 can affect any
  subset of its peripherals. If two unrelated faults show up on the
  same board, suspect the MCU package before suspecting two separate
  components.
- Crystals and their matching caps are surprisingly often the problem,
  especially small 3225 packages that get dragged out of place by
  neighbouring components during reflow.

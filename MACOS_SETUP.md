# macOS dev notes (not in upstream README)

Confirmed working on this Mac (Apple Silicon, Rust 1.97 stable, Homebrew sdl2-compat).

`.cargo/config.toml`'s default `[build] target` is `x86_64-unknown-linux-gnu`, which
doesn't exist on macOS. The `fw*` aliases override the target explicitly, but the
`sim` and `simulate-game` aliases don't — so `make sim` fails on macOS unless the
target is overridden via env var. Homebrew's SDL2 (`sdl2-compat`) also isn't on the
linker's default search path.

Export these before running any `cargo`/`make` command that doesn't target the
embedded chip (i.e. `make sim`, `make simulate-game`, plain `cargo build`/`test`):

```bash
export CARGO_BUILD_TARGET=aarch64-apple-darwin   # Apple Silicon; use x86_64-apple-darwin on Intel
export LIBRARY_PATH="/opt/homebrew/lib:$LIBRARY_PATH"
export DYLD_LIBRARY_PATH="/opt/homebrew/lib:$DYLD_LIBRARY_PATH"
```

Then `make sim` works as documented in the main [README.md](README.md#simulator).

Embedded builds (`make fw`, `make flash`, etc.) are unaffected — they already pass
`--target thumbv7em-none-eabihf` explicitly — but still need `arm-none-eabi-binutils`,
`probe-rs`, and (for USB DFU) `dfu-util`, none of which are installed yet.

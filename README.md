# Cyber AEgg Rust test firmware

## Setup

Clone the repository with submodules:

```bash
git clone --recursive <your-repo-url>
```

Install the Rust embedded target:

```bash
rustup target add thumbv7em-none-eabihf
```

## Build

### Simulator

The simulator requires SDL2:

```bash
# Debian / Ubuntu
sudo apt install libsdl2-dev

# Fedora
sudo dnf install SDL2-devel

# Arch Linux
sudo pacman -S sdl2

# macOS
brew install sdl2
```

Build and run:

```bash
make sim
```

#### Key bindings

| Key            | Action                                               |
|----------------|------------------------------------------------------|
| Arrow keys     | Navigate icons (up/down = row, left/right = column)  |
| Right at col 3 | Advance to next screen                               |
| Return         | Fire / open icon modal                               |
| Backspace      | Cancel / close modal                                 |
| E              | Execute button                                       |
| Escape         | Quit                                                 |

On the main menu screen, Up/Down scrolls items and Return selects.

### Firmware (nRF52840)

```bash
make fw
```

### Flash firmware

```bash
make flash
```

### Debug (VS Code + probe-rs)

Open the project in VS Code and press **F5**. The `cargo fw` build runs automatically before flashing.

### Other targets

| Command              | Description                              |
|----------------------|------------------------------------------|
| `make fw-release`    | Release build                            |
| `make flash-release` | Flash release build                      |
| `make monitor`       | Attach RTT log monitor (app)             |
| `make bl`            | Build bootloader                         |
| `make bl-flash`      | Full-chip erase + flash bootloader       |
| `make dfu-flash`     | Flash app over USB DFU                   |

## Python balance simulator

See [`simulation_py/README.md`](simulation_py/README.md) for the Bornpets game balance simulator.

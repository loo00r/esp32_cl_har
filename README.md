# esp32_cl_har

Continual Learning for Human Activity Recognition on ESP32 using a Rust-first firmware stack.

## Hardware

- **MCU**: ESP32-WROOM-32 (ESP32-D0WD-V3 rev3.1, 240 MHz, 320 KB SRAM, 4 MB Flash)
- **Sensor**: MPU6050 (GY-521) — SCL→GPIO22, SDA→GPIO21
- **Port**: `/dev/ttyUSB0`

## Prerequisites

### 1. Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Espressif Rust toolchain (Xtensa support)

```bash
cargo install espup
espup install
```

Add to `~/.bashrc` so it loads automatically:

```bash
echo '. $HOME/export-esp.sh' >> ~/.bashrc
```

For the current terminal session:

```bash
. $HOME/export-esp.sh
```

### 3. Flash tool

```bash
cargo install espflash
```

### 4. Add user to dialout (USB port access)

```bash
sudo usermod -a -G dialout $USER
# log out and back in for it to take effect
```

## Build & Flash

```bash
cargo build          # compile only
cargo run            # compile + flash + open serial monitor
cargo build --release
```

## Project Structure

```
esp32_cl_har/
├── src/
│   ├── bin/main.rs          # entry point, main loop
│   └── lib.rs               # shared library code
├── Cargo.toml               # dependencies
├── build.rs                 # linker configuration
├── rust-toolchain.toml      # pins to `esp` Xtensa toolchain
└── .cargo/config.toml       # target, runner (espflash), rustflags
```

## Project Rules

- `AGENTS.md` is the current agent/workflow contract
- `PLAN.md` is the technical and research source of truth
- Mainline firmware stays on Rust, not `C++/PlatformIO`

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `esp-hal` | Official Espressif HAL (GPIO, I2C, SPI, UART, timers) |
| `esp-bootloader-esp-idf` | ESP-IDF compatible bootloader descriptor |
| `critical-section` | Safe atomic operations (required by esp-hal) |

## Toolchain Notes

- Target: `xtensa-esp32-none-elf`
- Compiler: Espressif fork of Rust with Xtensa LLVM backend (`rustup toolchain: esp`)
- No standard library (`#![no_std]`, `#![no_main]`)

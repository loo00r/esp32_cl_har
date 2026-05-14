# esp32_cl_har

Rust-first continual learning for human activity recognition on an ESP32-WROOM-32 with an MPU6050 IMU.

This repository is the reproducible implementation behind a resource-focused ESP32 HAR study. The development process is documented step by step in [DEVLOG.md](DEVLOG.md); start there if you need the full research timeline, intermediate decisions, raw experiment notes, and validation history.

## What This Project Demonstrates

The project validates a minimal on-device continual learning pipeline for IMU-based human activity recognition:

```text
MPU6050 -> 80x3 accelerometer window -> INT8 MicroFlow-32 feature extractor
        -> Rust/no_std OnlineLayer32 -> RAM-only ReplayBuffer32
        -> prediction and supervised local update
```

The embedded runtime is intentionally small:

- frozen INT8 MicroFlow-32 feature extractor;
- trainable `OnlineLayer32` classifier updated on the ESP32 with mini-batch SGD;
- `ReplayBuffer32` stored in RAM only;
- FIFO and reservoir-per-class replay policies;
- no C++, Arduino, PlatformIO, heap-heavy training loop, or cloud upload path.

## Main Results

| Result | Value |
| --- | ---: |
| MicroFlow-64 feature extraction latency | 298.68 ms |
| MicroFlow-32 feature extraction latency | 172.01 ms |
| Latency reduction from 64 to 32 features | 42.4% |
| Replay buffer RAM, 64 features | 24 KiB |
| Replay buffer RAM, 32 features | 12 KiB |
| Online update cost, FIFO | 0.6665 ms |
| Online update cost, reservoir-per-class | 0.6560 ms |
| Update cost relative to frozen inference | less than 0.4% |
| MPU6050 target-motion accepted predictions, no adaptation | 0.0% |
| MPU6050 target-motion accepted predictions, FIFO | 88.57% |
| MPU6050 target-motion accepted predictions, reservoir-per-class | 93.94% |
| WISDM user 19 held-out accuracy, before adaptation | 73.03% |
| WISDM user 19 held-out accuracy, after adaptation | 80.26% |
| WISDM user 19 Downstairs recall, before adaptation | 0.00% |
| WISDM user 19 Downstairs recall, after adaptation | 78.57% |

The key engineering finding is that the online training step is not the bottleneck. The frozen feature extractor dominates runtime; the local weight update is sub-millisecond on the ESP32.

## Hardware

- MCU: ESP32-WROOM-32, ESP32-D0WD-V3 rev3.1, 240 MHz, 320 KiB SRAM, 4 MiB flash
- Sensor: MPU6050 / GY-521
- I2C wiring: `SCL -> GPIO22`, `SDA -> GPIO21`
- UART/flash port used during experiments: `/dev/ttyUSB0`
- Label input for supervised CL experiments: UART0 RX on `GPIO3`

## Repository Map

```text
src/
  bin/main.rs                         Main MPU6050 streaming firmware
  bin/wisdm_device_eval.rs            Device-side WISDM inference evaluation
  bin/wisdm_user19_device_cl.rs       Isolated target-user CL evaluation
  bin/*_smoke.rs                      Focused embedded smoke tests
  inference_microflow32.rs            MicroFlow-32 feature backend
  online_layer.rs                     OnlineLayer32 / OnlineLayer64
  replay_buffer.rs                    FIFO and reservoir replay buffers
  quant.rs                            Input quantization helpers
  window.rs                           Fixed-size accelerometer windowing
  model_artifacts/                    Tracked INT8 model artifacts
  eval_artifacts/                     Compact embedded evaluation artifacts

scripts/
  analysis/                           Tables, figures, and experiment summaries
  checks/                             Host-side consistency checks
  data/                               Dataset export and Rust artifact generation
  parsing/                            Serial-log parsers
  simulation/                         PC-side CL gates before firmware runs
  training/                           Fold-specific MicroFlow-32 training/export

logs/
  raw/                                Captured ESP32 serial logs
  parsed/                             Parsed experiment CSV/JSON summaries

results/
  figures/                            Paper figures
  tables/                             Paper and experiment tables
  fold_artifacts/                     Fold-specific target-user artifacts

notebooks/
  paper_results_analysis.ipynb        Final result analysis and figure generation

paper/                                Ukrainian article section drafts
DEVLOG.md                             Full chronological development log
PLAN.md                               Technical and research plan
THESIS.md                             Higher-level research framing
```

## Toolchain Setup

Install Rust and the Espressif Xtensa toolchain:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install espup
espup install
```

Load the ESP toolchain in each shell before building firmware:

```bash
. $HOME/export-esp.sh
```

Install the flasher:

```bash
cargo install espflash
```

Allow access to the USB serial device:

```bash
sudo usermod -a -G dialout $USER
```

Log out and back in after changing the `dialout` group.

## Firmware Commands

The repository has multiple binaries, so use explicit `--bin` names.

Build the main firmware:

```bash
. $HOME/export-esp.sh
cargo build --bin esp32_cl_har --features microflow32_backend,cl_uart_labels
```

Flash and monitor the main MPU6050 + MicroFlow-32 + CL firmware:

```bash
. $HOME/export-esp.sh
cargo run --bin esp32_cl_har --features microflow32_backend,cl_uart_labels
```

Run the same main firmware with FIFO replay instead of reservoir-per-class:

```bash
. $HOME/export-esp.sh
cargo run --bin esp32_cl_har --features microflow32_backend,cl_uart_labels,replay_fifo_policy
```

Run the no-adaptation baseline:

```bash
. $HOME/export-esp.sh
cargo run --bin esp32_cl_har --features microflow32_backend
```

Run the balanced WISDM device-side inference evaluation:

```bash
. $HOME/export-esp.sh
cargo run --bin wisdm_device_eval --features microflow32_backend
```

Run the WISDM user 19 target-user CL evaluation with reservoir-per-class:

```bash
. $HOME/export-esp.sh
cargo run --bin wisdm_user19_device_cl --features microflow32_backend
```

Run the same WISDM user 19 CL evaluation with FIFO:

```bash
. $HOME/export-esp.sh
cargo run --bin wisdm_user19_device_cl --features microflow32_backend,replay_fifo_policy
```

Useful embedded smoke tests:

```bash
. $HOME/export-esp.sh
cargo run --bin mpu_domain_shift_probe
cargo run --bin replay_buffer_smoke
cargo run --bin uart_label_smoke
cargo run --bin uart_replay_smoke
cargo run --bin microflow32_consistency_smoke --features microflow32_backend
```

## Host-Side Commands

Python is used only for dataset preprocessing, training/export, simulation, parsing, and plots. The firmware runtime remains Rust/no_std.

Compile all helper scripts:

```bash
find scripts -name '*.py' -print0 | xargs -0 python3 -m py_compile
```

Export compact WISDM device-evaluation artifacts:

```bash
python3 scripts/data/export_wisdm_device_eval_artifact.py
```

Audit target-user candidates:

```bash
python3 scripts/data/audit_wisdm_target_users.py
```

Train/export a fold-specific MicroFlow-32 model for a held-out user:

```bash
python3 scripts/training/train_wisdm_fold_microflow32.py --target-user 19 --epochs 10
```

Export a fold-specific Rust `OnlineLayer32` head:

```bash
python3 scripts/data/export_wisdm_target_user_head_rust.py --target-user 19
```

Run PC-side target-user CL simulation:

```bash
python3 scripts/simulation/simulate_wisdm_user7_cl.py --target-user 19 --budgets 5 10 20 --lrs 0.001 0.003 0.01
```

Run the balanced-600 PC-side CL gate:

```bash
python3 scripts/simulation/simulate_balanced600_cl.py --adaptation-per-class 20
```

Parse main firmware serial logs:

```bash
python3 scripts/parsing/parse_experiment_logs.py logs/raw/pilot_sit_up/sit_up_fifo_2026-05-09.txt --out-dir logs/parsed/pilot_sit_up/fifo
```

Parse WISDM device-side inference logs:

```bash
python3 scripts/parsing/parse_wisdm_device_eval.py logs/raw/wisdm_device_eval/wisdm_device_eval_balanced_600_2026-05-13.txt
```

Parse WISDM user 19 device-side CL logs:

```bash
python3 scripts/parsing/parse_wisdm_device_cl.py \
  logs/raw/wisdm_user19_device_cl/wisdm_user19_device_cl_fifo_2026-05-13.txt \
  logs/raw/wisdm_user19_device_cl/wisdm_user19_device_cl_reservoir_2026-05-13.txt
```

Build paper-ready WISDM user 19 tables and figures:

```bash
python3 scripts/analysis/build_wisdm_user19_device_cl_figures.py
```

Run the quantization sanity check:

```bash
python3 scripts/checks/quant_sanity_check.py
```

## Data And Artifacts

The raw WISDM dataset is expected under:

```text
data/WISDM_ar_v1.1/WISDM_ar_v1.1_raw.txt
```

`data/` is intentionally ignored because raw datasets are large and external. The `.tflite` files under `src/model_artifacts/` are tracked because they are firmware input artifacts, not runtime code.

## Reading The Results

Start with:

- [DEVLOG.md](DEVLOG.md) for the full chronological implementation and experiment history;
- [results/analysis_notes_uk.md](results/analysis_notes_uk.md) for interpretation boundaries and paper-safe claims;
- [results/tables/](results/tables/) for numerical outputs;
- [results/figures/](results/figures/) for generated plots;
- [paper/](paper/) for Ukrainian article drafts;
- [notebooks/paper_results_analysis.ipynb](notebooks/paper_results_analysis.ipynb) for final figure generation.

## Project Constraints

- Firmware stays Rust 2024, `no_std`, and `esp-hal`.
- Do not add C++, Arduino, PlatformIO, or `src/main.cpp` without an explicit project decision.
- Keep CL logic in small Rust modules with simple APIs.
- Minimize heap allocation and keep RAM/flash costs explicit.
- Treat Python as an offline research tool only.

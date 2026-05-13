# Draft Experimental Setup Section

Цей файл є робочим draft-ом секції `Experimental Setup` для статті про
`ESP32 CL-HAR`. Він описує апаратну платформу, firmware режими,
експериментальний protocol, логування, метрики та межі інтерпретації. Текст
спирається на `PLAN.md`, `DEVLOG.md`, `results/analysis_notes_uk.md` і вже
зібрані Phase 5 artifacts.

## Experimental Setup

### 1. Hardware platform

Експерименти виконувались на платі `ESP32-WROOM-32` з мікроконтролером
`ESP32-D0WD-V3`, ревізія `v3.1`. Плата працювала на частоті `240 MHz` і мала
`4 MB` Flash. Цільовим memory constraint для роботи є класичний
`ESP32-WROOM-32` без зовнішньої PSRAM, тобто приблизно `320 KB SRAM`.

Як IMU-сенсор використовувався модуль `MPU6050 / GY-521`, підключений через
I2C. У firmware використовувались такі підключення:

| Signal | ESP32 GPIO |
| --- | --- |
| I2C SDA | `GPIO21` |
| I2C SCL | `GPIO22` |
| UART logs / labels | USB serial / UART0 |

MPU6050 виявлявся за адресою `0x68`, а `WHO_AM_I` повертав `0x70`, що було
зафіксовано під час hardware smoke tests. Sampling path читав accelerometer
values на рівні low-level Rust driver без зовнішнього MPU6050 crate.

### 2. Firmware stack

Firmware реалізовано у стеку:

```text
Rust 2024
no_std
esp-hal
xtensa-esp32-none-elf
```

Проєкт не використовує `Arduino`, `PlatformIO` або C++ runtime у поточному
embedded path. Python використовується тільки для preprocessing, training,
quantization, parsing logs і plotting.

Ключові Rust-модулі:

| Module | Role |
| --- | --- |
| `src/mpu6050.rs` | low-level MPU6050 I2C access |
| `src/window.rs` | `SlidingWindow` для `80 x 3` samples |
| `src/quant.rs` | raw accelerometer counts -> physical units -> z-score -> int8 |
| `src/inference_microflow32.rs` | MicroFlow-32 frozen feature extraction |
| `src/online_layer.rs` | `OnlineLayer32` forward/backward |
| `src/replay_buffer.rs` | RAM-only `ReplayBuffer32`, FIFO/reservoir |
| `src/bin/main.rs` | boot, sensor loop, inference, optional CL loop |

### 3. Offline model preparation

Baseline model було підготовлено на `WISDM` dataset. Preprocessing pipeline
використовував fold-aware normalization, щоб уникнути leakage:

- `z-score` statistics рахувались тільки на train-subjects у межах fold;
- test-subject нормалізувався train statistics;
- windowing виконувався для вікон `80 x 3`;
- overlap становив `50%`;
- windowing не змішував різні `user_id`, `activity` або розірвані timestamp
  segments.

Початковий baseline мав архітектуру:

```text
Conv1D(32, kernel=5)
-> Conv1D(64, kernel=3)
-> GlobalAveragePooling1D
-> Dense(6, softmax)
```

Для embedded deployment було підготовлено MicroFlow-friendly full-conv export
path. Після latency/resource ablation основним embedded feature extractor став
`MicroFlow-32`, який формує `32` features для online head.

Цільовий embedded inference path:

```text
80 x 3 accelerometer window
-> int8[240]
-> MicroFlow-32 frozen feature extractor
-> f32[32] features
-> OnlineLayer32
```

`MicroFlow-64` лишається reference/stronger baseline, але не використовується як
основний path для Phase 5 CL experiments через вищу latency і вдвічі більший
replay RAM estimate.

### 4. Sensor preprocessing on ESP32

Firmware накопичує `80` accelerometer samples у `SlidingWindow`. Sampling loop
працює з цільовим періодом `50 ms`, тобто `20 Hz`. Для кожного sample MPU6050
повертає raw `i16` accelerometer counts.

У quantization path використано припущення, що після reset MPU6050 працює у
default accelerometer range `+-2g`, де scale становить `16384 LSB/g`. Перехід до
фізичних одиниць:

```text
g = raw / 16384.0
m/s^2 = g * 9.80665
```

Далі дані нормалізуються WISDM train statistics і квантуються у `int8` з
параметрами, отриманими з TFLite metadata:

```text
input_scale = 0.030599215999245644
input_zero_point = 9
```

Host-side quantization/layout sanity check показав `max_abs_diff = 0` і
`mismatch_count = 0` між Python reference і Rust-side simulated path для
`int8[240]` tensor layout.

### 5. Continual learning components

On-device continual learning реалізовано тільки для lightweight final head.
Frozen feature extractor не оновлюється на ESP32.

#### 5.1 OnlineLayer32

`OnlineLayer32` є trainable dense head:

```text
input:  f32[32]
output: f32[6]
```

Він підтримує:

- `forward()`;
- `forward_logits()`;
- `backward_batch()`;
- mini-batch SGD update.

Початкові weights `OnlineLayer32` не є zero-initialized у фінальному path. Вони
відновлені з classifier head шару `microflow_fullconv32_classifier_int8.tflite`,
тобто embedded runtime використовує:

```text
MicroFlow-32 frozen feature extractor
+ Rust OnlineLayer32 pretrained classifier head
```

#### 5.2 ReplayBuffer32

`ReplayBuffer32` є RAM-only storage:

```text
features[6][16][32]
seen[6]
len[6]
```

Параметри:

| Parameter | Value |
| --- | ---: |
| Classes | 6 |
| Slots per class | 16 |
| Feature dimension | 32 |
| Feature dtype | `f32` |
| Replay RAM estimate | `6 x 16 x 32 x 4 = 12288 B = 12 KiB` |

Підтримані policies:

- `FIFO`;
- `reservoir-per-class`.

Обидва policies використовують однаковий memory budget. Це важливо для чесного
порівняння: різниця між modes походить від replacement policy, а не від різного
розміру replay memory.

#### 5.3 Training schedule

Поточний CL schedule:

| Parameter | Value |
| --- | ---: |
| Labels per update `K` | 10 |
| Batch size | 12 |
| Learning rate | 0.001 |
| Persistence | off |
| Runtime flash writes | none |

Після кожних `K=10` supervised labels firmware формує replay mini-batch через
`ReplayBuffer32.sample_balanced_batch()` і виконує один
`OnlineLayer32.backward_batch()`.

### 6. Compared modes

Експерименти порівнюють три режими:

| Mode | Description |
| --- | --- |
| `no_adapt` | MicroFlow-32 + pretrained OnlineLayer32, без labels і без replay update |
| `FIFO` | UART labels + RAM-only ReplayBuffer32 з FIFO replacement |
| `reservoir` | UART labels + RAM-only ReplayBuffer32 з per-class reservoir replacement |

Firmware modes активуються через compile-time feature flags:

```text
no_adapt:
  cargo run --features microflow32_backend --bin esp32_cl_har

FIFO:
  cargo run --features microflow32_backend,cl_uart_labels,replay_fifo_policy --bin esp32_cl_har

reservoir:
  cargo run --features microflow32_backend,cl_uart_labels --bin esp32_cl_har
```

`cl_uart_labels` вмикає supervised labels і RAM-only CL loop. Без цього feature
firmware лишається inference-only.

### 7. Label protocol

Supervised labels надходять через USB serial / UART0 як single-character labels:

| Label | Class |
| ---: | --- |
| `0` | Walking |
| `1` | Jogging |
| `2` | Upstairs |
| `3` | Downstairs |
| `4` | Sitting |
| `5` | Standing |

Protocol навмисно простий:

- без JSON;
- без checksum;
- без timestamps;
- без binary protocol;
- без autonomous labeling.

Під час pilot-експериментів оператор надсилає labels відповідно до відомого
segment activity. Це моделює structured supervised feedback, а не повністю
автономний deployment scenario.

UART/USB використовується одночасно для logs і labels. Це обмежує природність
руху у pilot experiments, але не є центральною темою статті. У майбутній роботі
цей канал може бути замінений на BLE, локальний UI feedback або інший механізм.

### 8. Logging and parsing

Firmware друкує plain-text logs зі стабільними tags:

| Tag | Meaning |
| --- | --- |
| `EXPERIMENT` | mode, labels, policy, feature_dim, persistence |
| `RESOURCE` | replay RAM estimate, batch size, slots/class |
| `PRED` | prediction attempt, class, confidence, inference/head latency |
| `LABEL` | accepted supervised label, buffer state, push latency |
| `TRAIN` | train step, sample latency, update latency, buffer state |

Raw serial logs зберігаються у `logs/raw/`, а parsed outputs у `logs/parsed/`.
Для parsing використовуються scripts:

- `scripts/parse_experiment_logs.py`;
- `scripts/summarize_experiment_runs.py`;
- `scripts/evaluate_segments.py`;
- `scripts/build_pilot_results_tables.py`.

Paper-ready plots/tables генеруються у:

- `notebooks/paper_results_analysis.ipynb`;
- `results/figures/`;
- `results/tables/`.

### 9. Target-user WISDM device-side CL protocol

Окремий protocol використовується для чистішої WISDM continual-learning
перевірки. Він не використовує реальний `MPU6050` sensor path і не є UART dataset
streaming. Його мета - перевірити, чи може ESP32 виконати adaptation для
held-out WISDM target user.

Protocol:

```text
target_user=19
train users exclude target_user
fold-specific MicroFlow-32 feature extractor
fold-specific OnlineLayer32 head
target-user windows included as read-only int8 artifacts
pre-adaptation evaluation on held-out target-user windows
RAM-only replay adaptation on labeled target-user samples
post-adaptation evaluation on held-out target-user windows
```

Configuration:

| Parameter | Value |
| --- | ---: |
| Target windows | 208 |
| Adaptation labels | 56 |
| Held-out eval windows | 152 |
| Budget | 10 labels/class |
| Learning rate | 0.01 |
| Labels per update | 10 |
| Batch size | 12 |
| Replay slots/class | 16 |

Binary:

```text
src/bin/wisdm_user19_device_cl.rs
```

Logs:

```text
logs/raw/wisdm_user19_device_cl/
```

Parsed tables:

```text
results/tables/wisdm_user19_device_cl_summary.csv
results/tables/wisdm_user19_device_cl_per_class.csv
results/tables/wisdm_user19_device_cl_train.csv
```

Цей protocol підтримує target-user proof-of-concept claims, але не є повним
LOSO benchmark по всіх users.

### 10. Pilot protocol

Основний pilot для Results:

```text
Sitting vs upstairs-like vertical hand-motion
```

Protocol:

| Segment | Physical action | Expected label |
| --- | --- | ---: |
| Segment 1 | Sitting / stationary sensor | 4 |
| Segment 2 | Upstairs-like vertical hand-motion | 2 |

Для `no_adapt` labels не надсилались. Firmware тільки друкувала `PRED` logs.

Для `FIFO` і `reservoir` protocol:

```text
Sitting segment:
  send labels 4444444444
  wait for TRAIN step 1

Upstairs-like segment:
  send labels 2222222222
  wait for TRAIN step 2
  continue logging predictions
```

Другий segment не є real staircase benchmark. Він є upward hand-motion біля host
PC, обмежений USB-кабелем. Тому у Results використовується формулювання
`upstairs-like vertical hand-motion`.

Додатковий pilot:

```text
Sitting vs standing-like small movement
```

Він використовується лише як secondary sanity check і не подається як Walking
benchmark.

### 11. Metrics

Основні resource metrics:

- `MicroFlow-32` inference time;
- `OnlineLayer32.forward()` time;
- `ReplayBuffer32.push()` time;
- `ReplayBuffer32.sample_balanced_batch()` time;
- `OnlineLayer32.backward_batch()` update time;
- app size / Flash footprint;
- static RAM / estimated replay RAM;
- persistence state: off.

Основні pilot metrics:

- prediction class per attempt;
- prediction confidence per attempt;
- segment-level accepted prediction rate;
- prediction class distribution per segment;
- number of labels;
- number of train updates;
- train update attempts.

Segment-level accepted rate рахується не як фінальна HAR accuracy, а як
pilot-level agreement з очікуваними labels у вручну заданих attempt ranges.
Для upstairs-like segment accepted labels були `Upstairs` або `Downstairs`,
оскільки вертикальний hand-motion може правдоподібно активувати обидва
stair-like класи.

Основні target-user WISDM CL metrics:

- pre/post held-out accuracy;
- per-class held-out recall before/after adaptation;
- weak-class recall delta;
- number of adaptation labels;
- number of train updates;
- update latency;
- app/partition footprint.

### 12. Scope boundaries

Цей experimental setup підтримує такі claims:

- firmware pipeline працює end-to-end на реальному ESP32 + MPU6050;
- RAM-only `OnlineLayer32 + ReplayBuffer32` працюють на MCU;
- `FIFO` і `reservoir-per-class` можна порівнювати під однаковим memory budget;
- CL update overhead можна кількісно порівняти з frozen feature extraction;
- isolated WISDM target-user run показує held-out improvement для `user=19`;
- real-device pilot показує prediction shift після supervised labels.

Цей setup не підтримує такі claims:

- full `6-class HAR` accuracy benchmark на реальному сенсорі;
- full LOSO CL benchmark по всіх WISDM users;
- statistical superiority reservoir над FIFO;
- real staircase benchmark;
- autonomous label acquisition;
- persistence або flash wear evaluation;
- strict real-time `20 Hz` inference throughput у поточному synchronous path.

Ці межі важливі для коректної інтерпретації Results: робота показує feasibility,
resource profile і pilot-level adaptation behavior, а не завершену wearable HAR
product систему.

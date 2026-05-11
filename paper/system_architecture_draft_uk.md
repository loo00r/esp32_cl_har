# Draft System Architecture Section

Цей файл є робочим draft-ом секції `System Architecture` для статті про
`ESP32 CL-HAR`. Секція пояснює повний pipeline від offline training на PC до
RAM-only continual learning loop на ESP32.

## System Architecture

### 1. Overview

Запропонована система має split architecture: важка частина навчання і
підготовки feature extractor виконується offline на PC, а на ESP32 виконується
тільки inference frozen feature extractor і lightweight online adaptation
останнього шару.

Загальний pipeline:

```text
PC side:
  WISDM
  -> preprocessing / windowing 80x3
  -> CNN training
  -> INT8 quantization
  -> MicroFlow-compatible feature extractor artifacts

ESP32 side:
  MPU6050
  -> SlidingWindow 80x3
  -> quantize int8[240]
  -> frozen MicroFlow-32 feature extractor
  -> f32[32] features
  -> OnlineLayer32
  -> prediction
  -> optional supervised UART label
  -> ReplayBuffer32
  -> OnlineLayer32 mini-batch update
```

Ключова архітектурна ідея полягає в тому, що `MicroFlow-32` feature extractor
залишається frozen, а on-device learning обмежене легким `OnlineLayer32` і
RAM-only replay buffer. Це утримує on-device adaptation у межах ресурсів
`ESP32-WROOM-32`.

### 2. Offline PC pipeline

Offline pipeline готує модель і deployment artifacts. Вхідним dataset є
`WISDM`, що містить accelerometer readings для `6` HAR classes:

```text
Walking
Jogging
Upstairs
Downstairs
Sitting
Standing
```

Preprocessing виконує:

- очищення raw WISDM rows;
- mapping `activity -> label`;
- fold-aware `z-score` normalization без leakage;
- segmentation по безперервних temporal chunks;
- windowing `80 x 3`;
- overlap `50%`;
- формування train/test folds для `LOSO` evaluation.

Початковий baseline CNN:

```text
Input: 80 x 3
Conv1D(32, kernel=5, relu)
Conv1D(64, kernel=3, relu)
GlobalAveragePooling1D
Dense(6, softmax)
```

Цей baseline використовується для offline reference і для підготовки
deployment-oriented artifacts. Після compatibility work модель було переведено
у MicroFlow-friendly full-conv form, щоб уникнути unsupported або небажаних
TFLite ops.

### 3. Deployment-oriented feature extractor

Для embedded deployment використовуються full-conv `MicroFlow` artifacts.
Було перевірено два варіанти:

| Extractor | Output features | Role |
| --- | ---: | --- |
| `MicroFlow-64` | 64 | stronger/reference path |
| `MicroFlow-32` | 32 | primary ESP32 CL path |

Обидва variants мають clean MicroFlow-compatible op graph:

```text
CONV_2D
CONV_2D
AVERAGE_POOL_2D
```

Classifier artifact додатково має `1x1 Conv2D + Softmax` head, але в ESP32 CL
path classifier head відділено від frozen extractor і перенесено в Rust як
`OnlineLayer32`.

Практичне рішення:

- `MicroFlow-64` лишається reference/stronger baseline;
- `MicroFlow-32` є основним embedded path через нижчу latency і менший replay
  RAM estimate;
- `MicroFlow` не є scientific contribution, а лише practical Rust-first frozen
  feature extractor backend.

### 4. ESP32 sensor and preprocessing path

На ESP32 sensor path починається з `MPU6050`, підключеного через I2C.
Firmware читає тільки accelerometer axes `ax`, `ay`, `az`, оскільки target HAR
pipeline в цій роботі базується на accelerometer windows.

Streaming path:

```text
MPU6050 raw i16 accel
-> convert raw counts to m/s^2
-> WISDM z-score normalization
-> int8 quantization
-> int8[240] tensor
```

Sliding window:

```text
window length = 80 samples
axes = 3
sampling target = 20 Hz
input tensor = 80 x 3 x 1
flat tensor length = 240
```

Цей preprocessing path був окремо звірений з Python/TFLite reference. Для
fixed window host-side sanity check показав `max_abs_diff = 0` між Python
reference і Rust-side simulated quantization/layout path.

### 5. Frozen feature extraction on ESP32

ESP32 виконує feature extraction через `MicroFlow-32`:

```text
input:  int8[240]
output: f32[32]
```

Використовується `predict_quantized(i8)`, тобто firmware подає вже готовий
quantized input tensor. Це важливо для production-like path:

```text
normalized window
-> int8[240]
-> MicroFlow predict_quantized()
-> dequantized f32[32] features
```

PC TFLite vs ESP MicroFlow-32 consistency check підтвердив, що ESP32 рахує той
самий exported artifact: checksum і перші feature values збігались у межах
float formatting.

### 6. OnlineLayer32

`OnlineLayer32` є lightweight trainable classifier head:

```text
features: f32[32]
classes:  6
weights:  [32][6]
bias:     [6]
```

API:

```rust
forward(features) -> probabilities[6]
backward_batch(batch_features, batch_labels, lr)
```

У фінальному ESP32 path `OnlineLayer32` ініціалізується не нулями, а
pretrained weights, відновленими з quantized `MicroFlow-32` classifier head.
Таким чином offline-trained classifier розділено на:

```text
MicroFlow-32 frozen feature extractor: 80x3x1 -> f32[32]
Rust OnlineLayer32 pretrained head:   f32[32] -> class probabilities
```

Це розділення є центральним для CL design: frozen extractor лишається стабільним,
а адаптація виконується тільки в останньому шарі.

### 7. RAM-only ReplayBuffer32

Replay buffer тримає feature vectors, а не raw IMU windows. Це latent replay
підхід, адаптований до мінімального memory budget.

Layout:

```text
features[6][16][32]
seen[6]
len[6]
```

Memory estimate:

```text
6 classes x 16 slots/class x 32 features x 4 bytes = 12288 bytes = 12 KiB
```

Підтримані replay policies:

#### FIFO

`FIFO` є простим baseline. Для кожного класу нові samples замінюють старіші у
fixed-size class buffer. Це важливо як контрольна стратегія: вона проста,
дешева і очікувано може гірше зберігати різноманітність старих samples.

#### Reservoir-per-class

`reservoir-per-class` підтримує той самий fixed memory budget, але replacement
policy враховує загальну кількість побачених samples для кожного класу. Це має
краще наближати class-specific sample diversity без збільшення RAM.

У цій роботі FIFO і reservoir порівнюються під однаковим memory budget, тому
різниця між ними не походить від більшого буфера.

### 8. RAM-only CL loop

CL loop активується тільки за explicit feature flag `cl_uart_labels`.
Inference-only path лишається доступним окремо як `no_adapt`.

Runtime flow:

```text
1. Read MPU6050 accelerometer sample.
2. Update SlidingWindow.
3. When window is ready, quantize to int8[240].
4. Run MicroFlow-32 feature extractor.
5. Run OnlineLayer32.forward().
6. Print PRED log.
7. If UART label is available:
     add latest features + label to ReplayBuffer32.
8. Every K=10 labels:
     sample balanced replay mini-batch.
     run OnlineLayer32.backward_batch().
     print TRAIN log.
```

Training parameters:

```text
K = 10 labels/update
batch_size = 12
learning_rate = 0.001
persistence = off
```

No runtime CL state is written to flash. Weights and replay state live only in
RAM during the session.

### 9. Experiment modes as architecture variants

The same firmware architecture supports three experimental modes:

| Mode | Frozen extractor | OnlineLayer forward | UART labels | Replay | Update |
| --- | --- | --- | --- | --- | --- |
| `no_adapt` | yes | yes | no | no | no |
| `FIFO` | yes | yes | yes | FIFO | yes |
| `reservoir` | yes | yes | yes | reservoir-per-class | yes |

This allows a controlled comparison:

```text
No adaptation
vs
TinyOL-style last-layer update + FIFO replay
vs
TinyOL-style last-layer update + reservoir-per-class replay
```

The feature extractor, feature dimension, online head, learning rate, batch size
and replay memory budget remain fixed across FIFO and reservoir modes.

### 10. Logging architecture

The firmware emits stable text logs instead of a binary protocol. This keeps the
experiment pipeline simple and reproducible.

Main log tags:

```text
EXPERIMENT
RESOURCE
PRED
LABEL
TRAIN
```

These logs are parsed on PC into CSV/JSON files and then used by the analysis
notebook to generate tables and figures. This choice keeps embedded firmware
simple: the ESP32 does not need a filesystem, NVS state, binary protocol,
JSON parser or complex telemetry stack.

### 11. Architectural boundaries and contribution

The system intentionally keeps several boundaries clear:

#### What is fixed

- `MicroFlow-32` feature extractor;
- preprocessing constants;
- feature dimension `32`;
- number of classes `6`;
- replay budget `16` slots/class;
- RAM-only session state.

#### What is trainable on ESP32

- only `OnlineLayer32` weights and bias.

#### What is compared

- no adaptation;
- FIFO replay;
- reservoir-per-class replay.

#### What is out of scope

- updating the CNN feature extractor on ESP32;
- flash persistence for weights/replay;
- NVS storage;
- autonomous label acquisition;
- Bluetooth protocol;
- full wearable product UX;
- making MicroFlow itself the scientific contribution.

The contribution is therefore not a new CNN architecture or a new inference
runtime. The contribution is a resource-transparent embedded CL architecture:

```text
frozen feature extractor
+ Rust/no_std online head
+ RAM-only replay
+ FIFO/reservoir comparison
+ real MPU6050 pilot
+ resource measurements on ESP32
```

### 12. Why this architecture fits ESP32

This architecture is suitable for `ESP32-WROOM-32` because it avoids the most
expensive forms of on-device training:

- no convolutional backpropagation;
- no raw-window replay;
- no heap-heavy replay storage;
- no filesystem;
- no runtime flash writes;
- no PSRAM dependency.

The remaining adaptation cost is concentrated in a small dense head and a fixed
RAM replay buffer. The measured Results support this design choice: feature
extraction costs approximately `172 ms`, while RAM-only CL update costs about
`0.66 ms`. Therefore, the adaptation mechanism is not the dominant runtime
bottleneck; frozen feature extraction is.

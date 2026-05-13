# Структура статті: ESP32 CL-HAR

Цей файл фіксує робочу структуру статті українською мовою. Він спирається на
`PLAN.md`, `THESIS.md`, повний `DEVLOG.md`, `results/analysis_notes_uk.md`,
parsed CSV і вже згенеровані notebook-графіки.

Мета цього документа - перетворити devlog/results у статейну логіку без зміни
firmware, без нових hardware runs і без розширення scope.

## Центральна ідея

Головна ідея статті:

> Реалізація мінімального replay-based continual learning pipeline на
> `ESP32-WROOM-32` з реальним `MPU6050`, вимірюванням ресурсів і pilot-перевіркою
> адаптації на реальному IMU-сигналі.

Центр статті не USB, не кабель, не Bluetooth і не спроба зробити повний
`6-class HAR` benchmark. Центр статті - відтворюваний `Rust/no_std` pipeline:

```text
MPU6050
-> SlidingWindow 80x3
-> INT8 quantization
-> frozen MicroFlow-32 feature extractor
-> OnlineLayer32
-> RAM-only ReplayBuffer32
-> FIFO або reservoir-per-class
-> supervised UART labels
-> resource metrics + pilot prediction shift
```

## Головний claim

Обережне формулювання:

> We demonstrate that a minimal RAM-only replay-based continual learning pipeline
> for IMU-HAR can be implemented and profiled on an ESP32-WROOM-32 using a
> Rust/no_std firmware stack. In a real-device pilot, supervised FIFO and
> reservoir replay shifted predictions on an upstairs-like motion segment while
> keeping the online update cost below 1% of the frozen feature extraction time.
> In a staged WISDM target-user run, the same RAM-only adaptation improved
> held-out accuracy for user 19 from 73.03% to 80.26%.

Українська версія:

> Показано, що мінімальний RAM-only replay-based continual learning pipeline для
> IMU-HAR можна реалізувати й профілювати на ESP32-WROOM-32 у стеку Rust/no_std.
> У real-device pilot supervised FIFO та reservoir replay зміщували prediction
> distribution на upstairs-like segment, тоді як вартість online update лишалась
> меншою за 1% від часу frozen feature extraction. У staged WISDM target-user
> run ця сама RAM-only адаптація підняла held-out accuracy для `user=19` з
> 73.03% до 80.26%.

## Що не заявляємо

У тексті не можна стверджувати:

- що система досягає `SOTA` для HAR;
- що виконано повний `6-class HAR` benchmark на реальному сенсорі;
- що `upstairs-like` pilot є реальним staircase benchmark;
- що reservoir статистично кращий за FIFO;
- що система має autonomous labels;
- що реалізовано persistence/NVS/flash CL state;
- що `MicroFlow` є науковим внеском роботи.

## Рекомендована структура

### 1. Introduction

Задача секції: пояснити проблему й внесок без технічного перевантаження.

Ключові тези:

- HAR-моделі деградують при переході на нових користувачів, положення сенсора і
  інші апаратні умови.
- Cloud retraining або desktop retraining не завжди підходять для wearable/edge
  сценаріїв.
- ESP32-WROOM-32 має жорсткі обмеження: `320 KB SRAM`, без PSRAM у поточному
  target, обмежена latency.
- Більшість TinyML HAR робіт зупиняється на inference.
- Більш складні HAR+CL системи існують, але часто орієнтовані на сильніші MCU або
  складніші алгоритми.
- Ця робота захищає мінімальний, відтворюваний baseline: frozen extractor +
  trainable online head + RAM-only replay.

Що включити як contributions:

1. `Rust/no_std` реалізація ESP32 IMU-HAR pipeline з реальним `MPU6050`.
2. `OnlineLayer32` і `ReplayBuffer32` як RAM-only embedded CL modules.
3. Порівняння `no_adapt`, `FIFO`, `reservoir-per-class` під однаковим replay
   budget.
4. Resource profiling: inference latency, forward latency, update time, replay RAM,
   firmware footprint.
5. Device-side WISDM target-user CL proof-of-concept для `user=19`.
6. Real-device pilot, який показує prediction shift після supervised labels.

### 2. Related Work

Задача секції: поставити роботу між TinyML inference papers і складнішими
on-device CL systems.

Підсекції:

#### 2.1 TinyML HAR on microcontrollers

Що сказати:

- HAR на IMU часто використовує CNN/LSTM/DeepConvLSTM-подібні моделі.
- TinyML роботи добре покривають quantized inference, але рідше показують
  on-device adaptation.
- Наша робота не конкурує за максимальну offline accuracy, а фокусується на
  resource-constrained adaptation.

#### 2.2 Continual learning and online learning on embedded devices

Що сказати:

- TinyOL-like підхід: frozen backbone, trainable lightweight head.
- Replay потрібний для зменшення forgetting.
- FIFO є простим baseline.
- Reservoir-per-class дає більш контрольований memory budget per class.

#### 2.3 HAR continual learning references

Що сказати:

- `COOL`, `PACL+`, LifeLearner/Kwon-подібні системи використовуються як сильний
  фон.
- Вони можуть бути архітектурно складніші або орієнтовані на інші платформи.
- Наш внесок - простий ESP32 baseline з прозорими ресурсами.

Обов'язковий tone:

```text
We do not claim architectural superiority over these systems; instead, we provide
a smaller and reproducible baseline for the ESP32-class MCU.
```

### 3. System Architecture

Це має бути одна з найсильніших секцій.

Підсекції:

#### 3.1 Offline training pipeline

Включити:

- `WISDM`;
- fold-aware preprocessing без leakage;
- `80x3` windows, overlap `50%`;
- baseline `Conv1D(32) -> Conv1D(64) -> GAP -> Dense(6)`;
- LOSO baseline result як context;
- MicroFlow-friendly full-conv export path;
- `MicroFlow-32` як основний embedded artifact.

Числа:

- LOSO mean accuracy: `0.8130`;
- weighted F1: `0.8193`;
- найслабші класи: `Upstairs`, `Downstairs`;
- MicroFlow-32 classifier params: `3814`;
- MicroFlow-32 feature extractor params: `3616`;
- MicroFlow-32 feature artifact: приблизно `8.1 KB`;
- MicroFlow-32 classifier artifact: приблизно `9.4 KB`.

#### 3.2 Embedded inference path

Включити:

- `MPU6050` через I2C;
- sampling `20 Hz`;
- `SlidingWindow` на `80` samples;
- raw `i16 -> m/s^2 -> z-score -> int8[240]`;
- `MicroFlow-32 predict_quantized`;
- feature output `f32[32]`;
- pretrained `OnlineLayer32` head.

Числа:

- `MicroFlow-64`: `~298.7 ms`;
- `MicroFlow-32`: `~172.0 ms`;
- latency reduction: приблизно `42%`;
- `MicroFlow-32` app size у streaming path: близько `124-127 KB` залежно від build step.

#### 3.3 Online adaptation path

Включити:

- `OnlineLayer32`;
- `backward_batch()`;
- `K=10` labels/update;
- `batch_size=12`;
- `lr=0.001`;
- `ReplayBuffer32`;
- `16` slots/class;
- `6 x 16 x 32 x f32 = 12288 B = 12 KiB`;
- `FIFO` і `reservoir-per-class`;
- persistence off.

#### 3.4 Label acquisition

Формулювання:

> During pilot experiments, labels were provided by the operator through a simple
> UART interface. This is treated as structured supervised feedback, not as an
> autonomous labeling mechanism.

USB/UART згадати тут коротко. Не робити з цього окрему проблему.

### 4. Experimental Setup

Підсекції:

#### 4.1 Hardware and firmware

Включити:

- ESP32-WROOM-32, rev `v3.1`, `240 MHz`, `4 MB Flash`, `320 KB SRAM`;
- MPU6050/GY-521;
- I2C `GPIO21/GPIO22`;
- Rust 2024, `no_std`, `esp-hal`;
- no PSRAM;
- no runtime CL state writes to flash.

#### 4.2 Compared modes

Режими:

| Mode | Meaning |
| --- | --- |
| `no_adapt` | Frozen MicroFlow-32 + pretrained OnlineLayer32, no labels, no replay |
| `FIFO` | Supervised UART labels + FIFO replay, RAM-only |
| `reservoir` | Supervised UART labels + per-class reservoir replay, RAM-only |

#### 4.3 Pilot protocol

Головний pilot:

```text
Segment 1: Sitting / stationary, label 4
Segment 2: upstairs-like vertical hand-motion, label 2
Modes: no_adapt, FIFO, reservoir
```

Обов'язкове уточнення:

> The second segment approximates upward stair-like hand motion near the host PC;
> it is not a real staircase benchmark.

Другий pilot:

```text
Sitting vs standing-like small movement
```

Його використовувати як secondary sanity check, не як main result.

#### 4.4 Metrics

Метрики:

- inference latency;
- OnlineLayer forward time;
- CL update time;
- ReplayBuffer push/sample time;
- replay RAM estimate;
- firmware/app footprint;
- segment-level accepted prediction rate;
- prediction distribution by class.

### 5. Results

Це головна секція.

#### 5.1 Offline baseline and feature extractor selection

Включити:

- LOSO baseline `0.8130` mean accuracy;
- `Upstairs/Downstairs` weaker classes як мотивація;
- `MicroFlow-32` vs `MicroFlow-64` trade-off.

Графік бажаний:

- `MicroFlow-64 vs MicroFlow-32 latency`.

Якщо такого графіка ще немає, його можна зробити з DEVLOG values:

```text
MicroFlow-64 mean ~= 298.7 ms
MicroFlow-32 mean ~= 172.0 ms
```

#### 5.2 Runtime cost of RAM-only CL

Використати:

- `results/tables/table_resource_overhead_sit_up.md`
- `fig_inference_latency_sit_up`
- `fig_cl_update_cost_sit_up`

Ключовий текст:

> The online update cost was approximately `0.66 ms`, while frozen MicroFlow-32
> feature extraction took approximately `172 ms`. Therefore, the RAM-only CL
> update overhead stayed below `1%` of feature extraction latency.

Числа:

| Mode | Inference | Update | Update / inference | Replay RAM |
| --- | ---: | ---: | ---: | ---: |
| `FIFO` | `172.29 ms` | `666.5 us` | `0.387%` | `12 KiB` |
| `reservoir` | `172.32 ms` | `656.0 us` | `0.381%` | `12 KiB` |

#### 5.3 Real-device pilot: Sitting vs upstairs-like motion

Використати:

- `logs/parsed/pilot_sit_up/sit_up_segment_eval_2026-05-09.csv`
- `fig_segment_accepted_rate_sit_up`
- `fig_upstairs_like_shift_sit_up`

Ключова таблиця:

| Mode | Sitting accepted | Upstairs-like accepted | Main prediction counts on upstairs-like |
| --- | ---: | ---: | --- |
| `no_adapt` | `100.0%` | `0.0%` | `Sitting=27` |
| `FIFO` | `100.0%` | `88.57%` | `Sitting=4;Upstairs=31` |
| `reservoir` | `100.0%` | `93.94%` | `Downstairs=4;Sitting=2;Upstairs=27` |

Коректна інтерпретація:

- `no_adapt` не змінив клас на upstairs-like segment;
- FIFO/reservoir після labels `2` змістили predictions у `Upstairs/Downstairs`;
- result демонструє adaptation behavior, не фінальну accuracy.

#### 5.4 Prediction distribution shift

Використати:

- `fig_prediction_distribution_upstairs_like`
- майбутній attempt-level plot, якщо буде доданий.

Основна думка:

> The pilot shows a change in prediction distribution after supervised adaptation,
> rather than only a change in aggregate accepted rate.

#### 5.5 Secondary standing-like pilot

Використати коротко:

- `results/tables/phase5_pilot_results_2026-05-09.md`;
- `results/tables/table_optional_pilot_comparison.csv`.

Не роздувати. Один абзац:

> An earlier pilot with sitting and standing-like small movement confirmed the
> same logging and CL pipeline, but it is treated only as a secondary sanity
> check because the second segment was constrained by the USB connection and was
> not a full walking motion.

### 6. Discussion

Підсекції:

#### 6.1 What the results show

Тези:

- pipeline реально працює на ESP32;
- CL modules дешеві відносно inference;
- replay RAM `12 KiB` підходить для ESP32-class memory budget;
- supervised labels можуть змінювати prediction behavior на реальному IMU.

#### 6.2 What the results do not show

Тези:

- немає повного 6-class real-device benchmark;
- немає multi-subject власного dataset;
- немає statistical comparison FIFO vs reservoir;
- немає autonomous feedback;
- немає persistence.

#### 6.3 USB/UART limitation

Формулювання:

> In the current pilot setup, supervised labels and logs were exchanged through
> USB/UART. This constrained the naturalness of motion experiments and should be
> replaced by BLE, local UI feedback, or another feedback channel in future work.
> This limitation affects the pilot protocol, but not the central resource result:
> the RAM-only CL update overhead on ESP32 remained small relative to feature
> extraction.

Це вся роль USB у статті. Не більше.

#### 6.4 Why persistence is future work

Тези:

- persistence зміщує статтю в storage engineering;
- flash wear policy потребує окремого design/evaluation;
- поточна робота навмисно фокусується на RAM-only sessions.

#### 6.5 FIFO vs reservoir interpretation

Формулювання:

> Reservoir-per-class produced the highest accepted rate in the main pilot, but
> the current experiment is too small to claim statistical superiority. The
> stronger claim is that both FIFO and reservoir can be implemented under the same
> RAM budget and both produced prediction shifts after supervised labels.

### 7. Conclusion

Короткі тези:

- реалізовано ESP32 `Rust/no_std` CL-HAR pipeline;
- `MicroFlow-32` працює як frozen feature extractor;
- `OnlineLayer32` і `ReplayBuffer32` працюють на MCU;
- FIFO і reservoir реалізовані під однаковим memory budget;
- CL update overhead менший за `1%` від inference;
- real-device pilot показав prediction shift;
- future work: більший dataset, автономні labels, BLE/local feedback, persistence, ESP32-S3/PSRAM, довші experiments.

## Figure Plan

### Обов'язкові фігури

| Figure | Source | Статус | Роль |
| --- | --- | --- | --- |
| System architecture diagram | manual / draw.io / markdown-to-figure | треба зробити | пояснює pipeline |
| MicroFlow-64 vs MicroFlow-32 latency | `fig_microflow_latency_ablation` | готово | обґрунтовує вибір MicroFlow-32 |
| Inference latency on ESP32 | `fig_inference_latency_sit_up` | готово | resource result |
| CL update cost | `fig_cl_update_cost_sit_up` | готово | overhead result |
| Upstairs-like accepted rate | `fig_upstairs_like_shift_sit_up` | готово | main pilot result |
| Prediction distribution | `fig_prediction_distribution_upstairs_like` | готово | prediction shift evidence |

### Бажані фігури

| Figure | Source | Статус | Роль |
| --- | --- | --- | --- |
| Prediction class vs attempt | parsed `*_pred.csv` | готово | показує часовий shift |
| Confidence vs attempt | parsed `*_pred.csv` | готово | показує зміну confidence |
| Firmware footprint table/plot | DEVLOG app sizes | optional | resource completeness |

## Table Plan

### Обов'язкові таблиці

| Table | Source | Статус |
| --- | --- | --- |
| WISDM LOSO baseline summary | `DEVLOG.md` Phase 2d | треба оформити |
| MicroFlow-64 vs MicroFlow-32 resource comparison | `DEVLOG.md` Phase 3m/3p | треба оформити |
| RAM-only CL resource overhead | `table_resource_overhead_sit_up.md` | готово |
| Segment-level pilot result | `sit_up_segment_eval_2026-05-09.csv` | готово |
| Prediction distribution | `table_prediction_distribution_sit_up.csv` | готово |

## Найближчий наступний крок

Після цього outline attempt-level plots і MicroFlow latency ablation уже додані. Найкращий наступний маленький крок:

```text
Почати Results draft українською або англійською на основі готових tables/figures.
```

Після цього можна починати `Results` draft, бо буде достатньо:

- summary tables;
- bar charts;
- time/attempt plots;
- чітка структура статті;
- зафіксовані limitations.

## Заборонені наступні кроки в поточному sprint

- не змінювати firmware;
- не чіпати `main.rs`;
- не додавати Bluetooth;
- не додавати persistence;
- не перезапускати hardware experiments без окремого рішення;
- не робити USB центральною темою статті;
- не розширювати claims до full HAR benchmark.

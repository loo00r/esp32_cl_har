# Draft Results Section

Цей файл є робочим draft-ом секції `Results` для статті про `ESP32 CL-HAR`.
Текст написаний українською і спирається тільки на вже зібрані дані, таблиці та
графіки. Firmware, raw logs і експерименти не змінювались.

## Results

### 1. Offline baseline і вибір embedded feature extractor

Початковий offline baseline було навчено на `WISDM` з fold-aware preprocessing:
`z-score` статистики обчислювались лише на train-subjects у межах fold, а
windowing виконувався для вікон `80 x 3` з overlap `50%`. Базова архітектура
`Conv1D(32) -> Conv1D(64) -> GlobalAveragePooling -> Dense(6)` дала
`LOSO` mean accuracy `0.8130`, weighted F1 `0.8193` і macro F1 `0.7809`.
Найслабшими класами в offline evaluation були `Upstairs` і `Downstairs`, що
додатково мотивує перевірку адаптації на рухах, схожих на stair-like activity.

Для ESP32 deployment було підготовлено два `MicroFlow`-compatible full-conv
feature extractors: `MicroFlow-64` і легший `MicroFlow-32`. Обидва варіанти
працювали на реальному `ESP32-WROOM-32`, але latency/resource trade-off показав,
що `MicroFlow-32` краще відповідає цільовому embedded CL path.

**Figure:** `results/figures/fig_microflow_latency_ablation.png`

**Table:** `results/tables/table_microflow_latency_ablation.csv`

| Extractor | Feature dim | Mean latency | Replay RAM estimate |
| --- | ---: | ---: | ---: |
| `MicroFlow-64` | 64 | 298.683 ms | 24 KiB |
| `MicroFlow-32` | 32 | 172.017 ms | 12 KiB |

`MicroFlow-32` зменшив streaming feature extraction latency приблизно на
`42.4%` порівняно з `MicroFlow-64` і вдвічі зменшив оцінку replay RAM для
однакового replay layout. Тому `MicroFlow-32` використовується як основний
embedded feature extractor у подальших CL experiments, тоді як `MicroFlow-64`
лишається reference/stronger baseline.

### 2. Runtime cost RAM-only continual learning

Основний runtime pipeline на ESP32:

```text
MPU6050
-> SlidingWindow 80x3
-> INT8 quantization
-> MicroFlow-32 frozen feature extractor
-> OnlineLayer32
-> optional RAM-only replay update
```

Для режимів `FIFO` і `reservoir` використовувався однаковий replay budget:
`6` класів, `16` слотів на клас, `32` `f32` features на sample. Це дає
`6 x 16 x 32 x 4 = 12288 bytes`, тобто `12 KiB` RAM-only replay storage.
Persistence, NVS і runtime flash writes для CL state не використовувались.

**Figures:**

- `results/figures/fig_inference_latency_sit_up.png`
- `results/figures/fig_cl_update_cost_sit_up.png`

**Table:** `results/tables/table_resource_overhead_sit_up.csv`

| Mode | PRED rows | Labels | Train updates | Replay RAM | Inference mean | Head mean | Update mean | Update / inference |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `no_adapt` | 45 | 0 | 0 | 0 KiB | 172.512 ms | 106.111 us | - | - |
| `FIFO` | 50 | 20 | 2 | 12 KiB | 172.290 ms | 97.240 us | 666.5 us | 0.387% |
| `reservoir` | 50 | 20 | 2 | 12 KiB | 172.324 ms | 85.940 us | 656.0 us | 0.381% |

У всіх трьох режимах середня latency `MicroFlow-32` feature extraction
залишалась близькою до `172 ms`. Це показує, що основний runtime bottleneck у
поточному synchronous firmware path - frozen feature extractor, а не online CL
update. `OnlineLayer32.forward()` виконувався за десятки мікросекунд, а
`OnlineLayer32.backward_batch()` з replay mini-batch займав приблизно `0.66 ms`.

Головний ресурсний результат: RAM-only CL update overhead лишився нижчим за
`1%` від часу frozen feature extraction. Для `FIFO` overhead становив `0.387%`,
для `reservoir` - `0.381%`. Це підтримує тезу, що replay-based last-layer
adaptation є дешевою відносно inference частиною pipeline навіть на
`ESP32-WROOM-32`.

Водночас ці результати не означають strict real-time inference at `20 Hz`:
`172 ms` feature extraction time перевищує `50 ms` sampling period. Коректна
інтерпретація полягає в тому, що current firmware демонструє feasible
window-level CL pipeline, але synchronous feature extraction лишається основним
latency bottleneck.

### 3. Real-device pilot: Sitting vs upstairs-like vertical motion

Основний real-device pilot перевіряв не full `6-class HAR` accuracy, а
поведінку pipeline на реальному `MPU6050` сигналі. Pilot складався з двох
segmentів:

```text
Segment 1: Sitting / stationary, expected label = 4
Segment 2: upstairs-like vertical hand-motion, expected label = 2
```

Другий segment був upward hand-motion біля host PC, обмежений USB-кабелем. Його
треба інтерпретувати як `upstairs-like vertical hand-motion`, а не як реальний
staircase benchmark. Labels у CL runs надсилались оператором через UART як
supervised feedback.

**Figures:**

- `results/figures/fig_segment_accepted_rate_sit_up.png`
- `results/figures/fig_upstairs_like_shift_sit_up.png`

**Source table:** `logs/parsed/pilot_sit_up/sit_up_segment_eval_2026-05-09.csv`

| Mode | Sitting accepted rate | Upstairs-like accepted rate | Upstairs-like prediction counts |
| --- | ---: | ---: | --- |
| `no_adapt` | 100.0% | 0.0% | `Sitting=27` |
| `FIFO` | 100.0% | 88.57% | `Sitting=4;Upstairs=31` |
| `reservoir` | 100.0% | 93.94% | `Downstairs=4;Sitting=2;Upstairs=27` |

У `Sitting` segment всі три режими дали стабільні predictions як `Sitting`.
Це очікувано, бо pretrained `OnlineLayer32` уже коректно класифікував нерухомий
сенсор як `Sitting` у попередніх hardware smoke tests.

У `upstairs_like` segment режим `no_adapt` залишив усі predictions як `Sitting`
(`27/27`), хоча confidence помітно знижувався під час руху. Після supervised
labels `2`, обидва replay modes змістили prediction distribution у stair-like
класи. `FIFO` дав `31/35` accepted predictions на upstairs-like segment, а
`reservoir` дав `31/33` accepted predictions.

Цей результат показує не остаточну HAR accuracy, а зміну поведінки системи після
RAM-only supervised adaptation. Коректний висновок: `FIFO` і
`reservoir-per-class` обидва можуть бути реалізовані на ESP32 під однаковим
memory budget і обидва здатні змінювати prediction distribution на реальному
IMU-сигналі після supervised labels.

### 4. Prediction distribution і attempt-level dynamics

Агреговані accepted-rate результати доповнено графіками розподілу predicted
classes і attempt-level динаміки.

**Figures:**

- `results/figures/fig_prediction_distribution_upstairs_like.png`
- `results/figures/fig_prediction_class_attempt_sit_up.png`
- `results/figures/fig_confidence_attempt_sit_up.png`

**Tables:**

- `results/tables/table_prediction_distribution_sit_up.csv`
- `results/tables/table_attempt_level_events_sit_up.csv`

Attempt-level markers:

| Mode | Sitting attempts | Upstairs-like attempts | Label attempts | Train attempts |
| --- | --- | --- | --- | --- |
| `no_adapt` | 1-18 | 19-45 | - | - |
| `FIFO` | 1-15 | 16-50 | 8;9;26;27 | 9;27 |
| `reservoir` | 1-17 | 18-50 | 9;10;27;28 | 10;28 |

Prediction-over-attempt plots показують, що `no_adapt` лишається на класі
`Sitting` протягом усього upstairs-like segment. Для `FIFO` після переходу до
другого segment predictions стабільно переходять до `Upstairs`. Для `reservoir`
основна маса predictions також переходить до `Upstairs`, але періодично
з'являється `Downstairs`. Це правдоподібно для vertical hand-motion, тому що
`Upstairs` і `Downstairs` є близькими stair-like класами, а сам pilot не є
реальним stair-climbing benchmark.

Confidence plots уточнюють цю картину. У `no_adapt` confidence різко падає під
час upstairs-like motion, але predicted class не змінюється. У `FIFO` і
`reservoir` confidence також нижчий у motion segment, але predictions уже
зміщуються до stair-like classes після supervised labels і train updates. Це
підтримує інтерпретацію pilot як evidence of prediction shift, а не як
остаточний accuracy benchmark.

### 5. Secondary pilot: Sitting vs standing-like small movement

Попередній pilot `Sitting vs standing-like small movement` використовується лише
як secondary sanity check. Спочатку він планувався як `Sitting vs Walking`, але
фізично другий segment був standing-like small movement, обмежений USB-кабелем
біля host PC. Тому його не треба використовувати як Walking accuracy result.

**Reference table:** `results/tables/phase5_pilot_results_2026-05-09.md`

У цьому pilot `Sitting` segment також був стабільним у всіх режимах. На
standing-like segment `reservoir` показав сильніший prediction distribution
shift, ніж `FIFO`, але цей результат треба трактувати тільки як sanity evidence,
а не як статистичне доведення переваги reservoir.

### 6. Summary of result claims

Поточні результати підтримують такі обережні claims:

1. `MicroFlow-32` є практичнішим embedded feature extractor для ESP32 CL path,
   ніж `MicroFlow-64`, через `42.4%` нижчу latency і вдвічі менший replay RAM
   estimate.
2. RAM-only `OnlineLayer32` update з replay mini-batch коштує приблизно
   `0.66 ms`, що менше `1%` від `MicroFlow-32` feature extraction latency.
3. `ReplayBuffer32` з `16` slots/class потребує приблизно `12 KiB` RAM і
   працює для обох policy: `FIFO` і `reservoir-per-class`.
4. У real-device upstairs-like pilot `no_adapt` залишив second segment як
   `Sitting`, тоді як `FIFO` і `reservoir` змістили predictions у
   `Upstairs/Downstairs` після supervised labels.
5. Поточний pilot демонструє feasibility і prediction shift на реальному IMU,
   але не є повним `6-class HAR` benchmark і не доводить статистичну перевагу
   reservoir над FIFO.

### 7. Limitations visible from the results

Результати також показують кілька обмежень:

- synchronous `MicroFlow-32` inference є bottleneck і не забезпечує strict
  `20 Hz` inference throughput у поточній реалізації;
- labels і logs у pilot передавались через USB/UART, що обмежувало природність
  руху, але це є limitation експериментального стенду, а не центральна тема
  роботи;
- labels надсилались burst-ами, тому частина labels прив'язана до latest feature
  vector, а не рівномірно розподілена по всьому segment;
- pilot має коротку тривалість і не замінює multi-subject real-device HAR
  evaluation;
- persistence/NVS/flash storage не реалізовані навмисно і лишаються Future Work.

Ці обмеження не скасовують основний resource result: replay-based RAM-only
last-layer adaptation на ESP32 має малий update overhead порівняно з frozen
feature extraction і може змінювати prediction behavior на реальному IMU-сигналі.

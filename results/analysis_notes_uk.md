# Аналіз результатів Phase 5 для статті

Цей файл фіксує технічну інтерпретацію вже зібраних результатів для `ESP32 CL-HAR`.
Він не є фінальним текстом статті. Його задача - не втратити числові деталі з `DEVLOG.md`,
CSV-таблиць і notebook-графіків під час підготовки розділів `Experimental Setup`,
`Results` і `Discussion`.

## Межі інтерпретації

Поточні результати треба описувати як:

- feasibility study для minimal RAM-only replay-based continual learning на ESP32;
- real-device pilot sanity check на `ESP32-WROOM-32 + MPU6050`;
- resource-oriented comparison `no_adapt` vs `FIFO` vs `reservoir-per-class`;
- demonstration of prediction shift після supervised UART labels.

Поточні результати не треба описувати як:

- повний `6-class HAR` benchmark;
- `SOTA` accuracy result;
- real staircase benchmark;
- автономну систему отримання міток;
- дослідження flash persistence або flash wear;
- роботу про `MicroFlow` як основний scientific contribution.

Основна наукова рамка лишається така:

```text
Frozen MicroFlow-32 feature extractor
-> Rust/no_std OnlineLayer32
-> RAM-only ReplayBuffer32
-> FIFO vs reservoir-per-class
-> supervised UART labels
-> resource-constrained evaluation on ESP32
```

## Джерела даних

### Основний pilot

`Sitting vs upstairs-like vertical hand-motion`

Файли:

- `logs/raw/pilot_sit_up/sit_up_no_adapt_2026-05-09.txt`
- `logs/raw/pilot_sit_up/sit_up_fifo_2026-05-09.txt`
- `logs/raw/pilot_sit_up/sit_up_reservoir_2026-05-09.txt`
- `logs/parsed/pilot_sit_up/sit_up_comparison_2026-05-09.csv`
- `logs/parsed/pilot_sit_up/sit_up_segment_eval_2026-05-09.csv`

Це найсильніший real-device pilot у поточному наборі результатів.
Другий segment був upward hand-motion біля ПК, обмежений USB-кабелем.
Його можна описувати як `upstairs-like vertical hand-motion`, але не як реальний stair-climbing benchmark.

### Додатковий pilot

`Sitting vs standing-like small movement`

Файли:

- `logs/raw/pilot_2class/no_adapt_pilot_2class_2026-05-09.txt`
- `logs/raw/pilot_2class/fifo_pilot_2class_2026-05-09.txt`
- `logs/raw/pilot_2class/reservoir_pilot_2class_2026-05-09.txt`
- `logs/parsed/pilot_2class/pilot_2class_comparison_2026-05-09.csv`
- `logs/parsed/pilot_2class/pilot_2class_segment_eval_2026-05-09.csv`

Цей pilot треба використовувати як secondary sanity check.
Його не треба подавати як `Walking` result, бо фізично рух був standing-like/small movement.

### Notebook outputs

Файл:

- `notebooks/paper_results_analysis.ipynb`

Згенеровані таблиці:

- `results/tables/table_resource_overhead_sit_up.csv`
- `results/tables/table_resource_overhead_sit_up.md`
- `results/tables/table_prediction_distribution_sit_up.csv`
- `results/tables/table_optional_pilot_comparison.csv`
- `results/tables/phase5_pilot_results_2026-05-09.md`

Згенеровані графіки:

- `results/figures/fig_inference_latency_sit_up.png`
- `results/figures/fig_inference_latency_sit_up.pdf`
- `results/figures/fig_cl_update_cost_sit_up.png`
- `results/figures/fig_cl_update_cost_sit_up.pdf`
- `results/figures/fig_segment_accepted_rate_sit_up.png`
- `results/figures/fig_segment_accepted_rate_sit_up.pdf`
- `results/figures/fig_upstairs_like_shift_sit_up.png`
- `results/figures/fig_upstairs_like_shift_sit_up.pdf`
- `results/figures/fig_prediction_distribution_upstairs_like.png`
- `results/figures/fig_prediction_distribution_upstairs_like.pdf`

## Основні числові результати

### Resource and CL overhead

Джерело: `results/tables/table_resource_overhead_sit_up.md`

| Mode | PRED rows | Labels | Train updates | Replay RAM | Inference mean | Head mean | Update mean | Update / inference |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `no_adapt` | 45 | 0 | 0 | 0 KiB | 172.51 ms | 106.11 us | - | - |
| `fifo` | 50 | 20 | 2 | 12.0 KiB | 172.29 ms | 97.24 us | 666.5 us | 0.387% |
| `reservoir` | 50 | 20 | 2 | 12.0 KiB | 172.32 ms | 85.94 us | 656.0 us | 0.381% |

Технічна інтерпретація:

- `MicroFlow-32` є основним latency bottleneck: приблизно `172 ms` на window.
- `OnlineLayer32.forward()` має порядок `86-106 us`, тобто суттєво дешевший за feature extraction.
- RAM-only CL update має порядок `0.66 ms`.
- CL update overhead у цьому pilot менший за `0.4%` від MicroFlow-32 inference time.
- Replay RAM для `feature_dim=32`, `6` класів і `16` slots/class становить `12288 B`, тобто `12 KiB`.

Коректний claim для статті:

```text
The online update cost remained below 1% of the frozen MicroFlow-32 feature extraction latency.
```

Не варто писати:

```text
The system is real-time at 20 Hz.
```

Причина: `172 ms` inference довший за `50 ms` sampling period для strict `20 Hz` continuous inference.
Коректніше писати, що система є feasible для window-level pilot evaluation, але synchronous MicroFlow inference є bottleneck.

### Segment-level prediction agreement

Джерело: `logs/parsed/pilot_sit_up/sit_up_segment_eval_2026-05-09.csv`

| Mode | Segment | Attempts | Accepted labels | Rows | Accepted rate | Prediction counts |
| --- | --- | --- | --- | ---: | ---: | --- |
| `no_adapt` | sitting | 1-18 | Sitting | 18 | 100.0% | Sitting=18 |
| `no_adapt` | upstairs_like | 19-45 | Upstairs\|Downstairs | 27 | 0.0% | Sitting=27 |
| `fifo` | sitting | 1-15 | Sitting | 15 | 100.0% | Sitting=15 |
| `fifo` | upstairs_like | 16-50 | Upstairs\|Downstairs | 35 | 88.57% | Sitting=4;Upstairs=31 |
| `reservoir` | sitting | 1-17 | Sitting | 17 | 100.0% | Sitting=17 |
| `reservoir` | upstairs_like | 18-50 | Upstairs\|Downstairs | 33 | 93.94% | Downstairs=4;Sitting=2;Upstairs=27 |

Технічна інтерпретація:

- Stationary `Sitting` стабільно розпізнається у всіх трьох режимах.
- У `no_adapt` другий segment повністю лишився `Sitting`, хоча фізично був upward hand-motion.
- Після supervised labels `2` FIFO і reservoir змістили predictions у stair-like класи.
- FIFO дав `31/35` accepted predictions на upstairs-like segment.
- Reservoir дав `31/33` accepted predictions на upstairs-like segment.
- Reservoir у цьому pilot трохи вищий за FIFO, але sample занадто малий для statistical superiority claim.

Коректний claim:

```text
Both replay modes shifted the prediction distribution toward upstairs/downstairs classes after supervised labels.
```

Обережний claim:

```text
Reservoir showed a slightly higher accepted rate in this pilot, but the experiment is too small to establish superiority.
```

Некоректний claim:

```text
Reservoir is better than FIFO for HAR on ESP32.
```

## Інтерпретація кожного графіка

### `fig_inference_latency_sit_up`

Що показує:

- середню latency `MicroFlow-32` feature extraction для `no_adapt`, `fifo`, `reservoir`;
- усі режими тримаються близько `172 ms`;
- CL mode майже не впливає на frozen extractor latency.

Як використовувати:

- як resource figure для inference cost;
- поруч із текстом про те, що inference є dominant runtime cost.

Що не показує:

- не показує end-to-end real-time throughput;
- не показує accuracy.

### `fig_cl_update_cost_sit_up`

Що показує:

- `OnlineLayer32.backward_batch()` update cost для `FIFO` і `reservoir`;
- обидва режими приблизно `0.66 ms`;
- різниця між FIFO і reservoir у train update cost мала.

Як використовувати:

- як ключовий графік для claim: CL overhead low relative to inference.

Що не показує:

- не показує повний cost label acquisition;
- не показує довготривалу стабільність після багатьох update.

### `fig_segment_accepted_rate_sit_up`

Що показує:

- accepted prediction rate для `sitting` і `upstairs_like` segments;
- `sitting` стабільний у всіх режимах;
- `upstairs_like` покращується після adaptation.

Як використовувати:

- як real-device pilot validation figure;
- з чітким підписом, що це segment-level agreement, не фінальна accuracy.

### `fig_upstairs_like_shift_sit_up`

Що показує:

- тільки upstairs-like segment;
- `no_adapt` near `0`;
- `fifo` high;
- `reservoir` high/slightly higher.

Це найсильніший графік для Results.

Коректний caption:

```text
Segment-level accepted prediction rate on the upstairs-like vertical hand-motion pilot.
The segment approximates upward stair-like motion near the host PC and is not a staircase benchmark.
```

### `fig_prediction_distribution_upstairs_like`

Що показує:

- розподіл predicted classes на upstairs-like segment;
- `no_adapt`: всі predictions у `Sitting`;
- `fifo`: переважно `Upstairs`;
- `reservoir`: переважно `Upstairs`, частково `Downstairs`.

Як використовувати:

- як evidence не тільки accepted rate, а саме prediction distribution shift;
- корисний для Discussion про те, що `Upstairs` і `Downstairs` можуть бути близькі для vertical hand motion.

## Що вже можна переносити в статтю

### Experimental Setup

Факти, які вже стабільні:

- hardware: `ESP32-WROOM-32`, `MPU6050/GY-521`, I2C `GPIO21/GPIO22`;
- firmware: `Rust 2024`, `no_std`, `esp-hal`;
- sampling/windowing: `20 Hz`, `80x3` windows;
- frozen extractor: `MicroFlow-32`, INT8, output `f32[32]`;
- head: `OnlineLayer32`, pretrained from MicroFlow-32 classifier artifact;
- CL: supervised UART labels, `K=10`, `batch_size=12`, `lr=0.001`;
- replay: RAM-only, `6 x 16 x 32 x f32 = 12 KiB`;
- modes: `no_adapt`, `FIFO`, `reservoir-per-class`;
- persistence: off/deferred.

### Results

Факти, які вже можна формулювати:

- `MicroFlow-32` inference on ESP32: приблизно `172 ms`;
- `MicroFlow-64` reference latency: приблизно `298.7 ms`;
- `MicroFlow-32` зменшив latency приблизно на `42%` проти `MicroFlow-64`;
- CL update cost: приблизно `0.66 ms`;
- update overhead: приблизно `0.38%`;
- replay RAM: `12 KiB`;
- `Sitting` segment стабільний;
- upstairs-like segment:
  - `no_adapt`: `0.0%`;
  - `FIFO`: `88.57%`;
  - `reservoir`: `93.94%`.

### Discussion

Факти, які треба обговорити:

- головний runtime bottleneck - frozen feature extractor, не CL update;
- domain shift між WISDM і MPU6050 підтверджено probe-ом;
- supervised UART labels є experimental feedback channel, не deployment-ready UX;
- persistence винесено у Future Work, щоб не змішувати CL baseline з flash wear engineering;
- upward hand-motion pilot не замінює повний multi-subject HAR benchmark;
- FIFO/reservoir comparison поки pilot-level, не statistical conclusion.

## Що ще бракує

### Для поточного короткого paper/result draft

Мінімально достатньо:

- використати вже згенеровані графіки з `results/figures`;
- вставити resource table з `results/tables/table_resource_overhead_sit_up.md`;
- вставити segment-level table з `sit_up_segment_eval_2026-05-09.csv`;
- додати абзац про limitations.

### Для сильнішої статті

Бажано, але не обов'язково для поточного sprint:

- повторити `Sitting vs upstairs-like` pilot 2-3 рази для variance;
- провести короткий real movement pilot з більш вільним USB/живленням;
- зробити більш рівномірний label timing, а не burst labels;
- attempt-level plot: class prediction vs attempt для кожного режиму — зроблено;
- confidence vs attempt plot — зроблено;
- зробити окрему table для firmware Flash footprint:
  - `no_adapt` приблизно `127408 B / 3.09%`;
  - CL build приблизно `136384 B / 3.30%`;
- інструментувати peak stack/high-water mark, якщо буде час.

## Найближчий практичний крок

Найкращий наступний маленький крок:

```text
Почати Results draft на основі вже готових таблиць і графіків.
```

Чому це важливо:

- resource/pilot графіки вже закриті;
- є latency ablation, CL overhead, accepted-rate, prediction-distribution і attempt-level plots;
- наступна цінність тепер не в нових firmware features, а в акуратному тексті `Results`.

Чого не треба робити наступним кроком:

- не змінювати firmware;
- не додавати persistence;
- не робити новий UART protocol;
- не бігти в повний 6-class experiment, поки поточні графіки не оформлені.

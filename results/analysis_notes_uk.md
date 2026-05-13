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

### Device-side WISDM subset evaluation

Файли:

- `logs/raw/wisdm_device_eval/wisdm_device_eval_smoke_120_2026-05-13.txt`
- `logs/raw/wisdm_device_eval/wisdm_device_eval_balanced_600_2026-05-13.txt`
- `results/tables/wisdm_device_eval_summary.csv`
- `results/tables/wisdm_device_eval_per_class_smoke_120.csv`
- `results/tables/wisdm_device_eval_confusion_smoke_120.csv`
- `results/tables/wisdm_device_eval_per_class_balanced_600.csv`
- `results/tables/wisdm_device_eval_confusion_balanced_600.csv`

Це device-side WISDM subset sanity evaluation. Вона перевіряє, що embedded
`MicroFlow-32 + OnlineLayer32` inference path може обробляти відомі WISDM windows
без PC-side inference. Це не CL experiment, не UART dataset streaming, не ReplayBuffer
і не full `LOSO CV` на ESP32.

`balanced_600` є першим paper-safe WISDM subset результатом:

```text
total=600
distribution=100/class x 6
correct=478
accuracy=79.67%
macro_recall=79.67%
mean_infer_us=171714
min_infer_us=171306
max_infer_us=173165
app_partition_usage=245120/4128768 bytes = 5.94%
```

Per-class recall для `balanced_600`:

```text
Walking    98%
Jogging    96%
Upstairs   71%
Downstairs 25%
Sitting    88%
Standing   100%
```

Коректний claim:

```text
The ESP32 device-side MicroFlow-32 + OnlineLayer32 inference path processed a balanced 600-window WISDM subset with 79.67% accuracy and 171.7 ms mean inference latency.
```

Некоректний claim:

```text
This is a full on-device LOSO benchmark.
```

### Target-user CL direction

Після перевірки training notebook стало ясно, що поточний deployed
`MicroFlow-32` artifact тренувався на `X_final` з усього WISDM corpus. Тому
`balanced_600` не можна трактувати як user-held-out evaluation.

Для справжнього WISDM CL experiment потрібен окремий fold-specific path:

```text
target user held out
-> train MicroFlow-32 без target user
-> export fold-specific feature extractor/head/z-score
-> generate target-user windows
-> ESP32 pre-adaptation evaluation
-> supervised RAM-only adaptation
-> ESP32 post-adaptation evaluation
```

PC-only audit target users:

- `results/tables/wisdm_target_user_audit.csv`

Найкращий поточний кандидат:

```text
user=7
total_windows=303
class_coverage=6
Walking=121
Jogging=89
Upstairs=41
Downstairs=13
Sitting=10
Standing=29
```

Це не означає, що треба одразу робити повний 6-class CL benchmark. Практичний
перший target-user experiment може бути обмежений класами з достатнім support,
або використовувати всі 6 класів з чесним застереженням про малий support для
`Sitting` і `Downstairs`.

Fold-specific export для `target_user=7` уже доступний:

- `results/fold_artifacts/wisdm_user7_microflow32/`
- `results/tables/wisdm_fold_user7_microflow32_summary.csv`

Ключові PC-side результати:

```text
train users: all except user 7
train_windows=8851
target_windows=303
representative_samples=192
final_train_accuracy=86.09%
final_val_accuracy=78.89%
target_keras_accuracy=91.09%
target_tflite_accuracy=89.11%
```

Target-user `user=7` TFLite per-class recall:

```text
Walking    100.00%
Jogging     94.38%
Upstairs    58.54%
Downstairs  15.38%
Sitting    100.00%
Standing   100.00%
```

Інтерпретація:

- `user=7` є методично чистим held-out target user, бо fold-specific model training
  виключає цього користувача.
- Overall target accuracy уже висока (`89.11%`), тому великий gain у total accuracy
  може бути обмежений.
- Науково цікавіший сигнал для target-user CL тут - чи supervised adaptation
  покращує слабкі stair-like класи, особливо `Downstairs`, а не просто overall accuracy.

PC-side CL simulation для `user=7`:

- `results/tables/wisdm_user7_pc_cl_simulation.csv`

Симуляція повторює поточний малий ESP32 CL loop:

```text
feature_dim=32
slots_per_class=16
labels_per_update=10
batch_size=12
learning_rate=0.001
policies=FIFO/reservoir
```

Результат gate:

```text
no_adapt all:
  accuracy=90.10%
  Upstairs recall=60.98%
  Downstairs recall=30.77%

budget=3/class:
  no_adapt_eval_split accuracy=90.18%
  FIFO accuracy=90.18%
  reservoir accuracy=90.18%

budget=5/class:
  no_adapt_eval_split accuracy=90.11%
  FIFO accuracy=90.11%
  reservoir accuracy=90.11%

budget=10/class:
  no_adapt_eval_split accuracy=90.98%
  FIFO accuracy=90.98%
  reservoir accuracy=90.98%
```

Інтерпретація:

- За поточним `lr=0.001` і `K=10` OnlineLayer adaptation майже не змінює
  predictions для `user=7`.
- `FIFO` і `reservoir` не відрізняються на цих малих budgets, бо рідкі класи
  не перевищують replay capacity.
- Device-side target-user CL run поки не варто подавати як expected improvement
  experiment. Перед ESP32 інтеграцією треба або змінити adaptation protocol
  на PC, або трактувати цей напрям як negative gate.

Додатково перевірено `target_user=20`, бо він має значно кращий support для
`Sitting/Walking`:

```text
target_user=20
target_windows=355
Walking=102
Upstairs=46
Downstairs=50
Sitting=90
Standing=67
Jogging=0
```

Fold-specific baseline:

```text
target_tflite_accuracy=91.83%
Walking recall=100.00%
Sitting recall=94.44%
Downstairs recall=60.00%
```

PC-side CL simulation:

```text
budget=5/class:
  no_adapt_eval_split accuracy=93.33%
  FIFO/reservoir accuracy=93.33%
  Downstairs recall: 66.67% -> 68.89%

budget=10/class:
  no_adapt_eval_split accuracy=94.75%
  FIFO/reservoir accuracy=94.43%
  Downstairs recall: 75.00% -> 77.50%

budget=20/class:
  no_adapt_eval_split accuracy=99.22%
  FIFO/reservoir accuracy=98.04%
  Downstairs recall: 93.33% -> 100.00%
```

Інтерпретація:

- `user=20` не підходить для демонстрації покращення `Sitting/Walking`, бо ці класи
  вже мають дуже високий baseline recall.
- CL трохи піднімає `Downstairs`, але не покращує overall accuracy.
- Це підтверджує, що WISDM target-user CL result не треба поспіхом переносити
  на ESP32 без окремого protocol redesign або пошуку іншого target scenario.

### Multi-user target screening

Після негативних/слабких gates для `user=7` і `user=20` виконано PC-only
screening кількох target users. Це не device-side experiment і не `full_9154`
run. Мета - знайти held-out user, де baseline має достатньо слабке місце, а
RAM-only head adaptation може дати числовий gain без train-on-test leakage.

Виконані users:

```text
user=7
user=8
user=19
user=20
user=27
```

Для кожного user:

```text
train fold-specific MicroFlow-32 без target user
evaluate target-user baseline
run PC-side FIFO/reservoir adaptation simulation
budgets: 5 / 10 / 20 labels per class
learning rates: 0.001 / 0.003 / 0.01
held-out evaluation split after adaptation sample selection
```

Зведений файл:

- `results/tables/wisdm_target_user_cl_screening_summary.csv`

Найкращий positive gate:

```text
target_user=19
target_windows=208
baseline target_tflite_accuracy=71.15%

budget=10/class
lr=0.01
policy=FIFO або reservoir

held-out no_adapt accuracy=73.03%
held-out adapted accuracy=80.26%
overall gain=+7.24 percentage points
macro recall gain=+13.10 percentage points
Downstairs recall: 0.00% -> 78.57%
```

Для `budget=5/class`, `lr=0.01`:

```text
accuracy: 71.35% -> 75.28%
overall gain=+3.93 pp
Downstairs recall: 0.00% -> 42.11%
```

Для `budget=20/class`, `lr=0.01`:

```text
reservoir accuracy: 76.47% -> 80.39%
overall gain=+3.92 pp
Downstairs recall: 0.00% -> 100.00%
```

Інтерпретація:

- `user=19` є першим target-user WISDM scenario, де поточний lightweight
  OnlineLayer update показує meaningful PC-side gain.
- Gain не походить від змішаного `balanced_600`; train users exclude target user.
- Сигнал сконцентрований у слабкому stair-like класі `Downstairs`, тоді як
  `Walking/Jogging/Sitting/Standing` уже були сильними.
- FIFO і reservoir однакові або дуже близькі в цьому PC gate, бо поточний порядок
  adaptation samples і replay capacity ще не створюють сильного розходження між
  policies.
- Це достатня підстава готувати isolated ESP32 target-user CL binary для
  `user=19`, але не підстава змінювати `main.rs` або запускати `full_9154`.

PC-only `balanced_600` CL split gate:

- `results/tables/wisdm_balanced600_pc_cl_split_20perclass.csv`

Протокол:

```text
source: existing balanced_600 artifact
adaptation: first 20 windows/class = 120 labeled windows
evaluation: remaining 80 windows/class = 480 held-out windows
no overlap between adaptation and evaluation
lr=0.001
labels_per_update=10
batch_size=12
updates=12
```

Результат:

```text
no_adapt:
  accuracy=79.58%
  Upstairs recall=75.00%
  Downstairs recall=23.75%

FIFO:
  accuracy=80.21%
  Upstairs recall=77.50%
  Downstairs recall=26.25%

reservoir:
  accuracy=80.21%
  Upstairs recall=77.50%
  Downstairs recall=26.25%
```

Інтерпретація:

- Це перший clean PC gate, де CL дає невеликий positive signal без train-on-test leakage.
- Приріст малий: `+0.63 pp` overall accuracy, `+2.5 pp` для `Upstairs` і `Downstairs`.
- FIFO і reservoir однакові, бо `20/class` adaptation майже не створює різниці між replay policies при `16 slots/class`.
- Цей результат може виправдати короткий device-side staged run, але не як сильний
  фінальний accuracy claim. Його правильна роль - показати, що CL direction на WISDM
  має слабкий, але реальний signal; основний paper claim все одно лишається resource/feasibility.

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

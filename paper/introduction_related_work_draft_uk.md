# Draft Introduction And Related Work

Цей файл є робочим draft-ом секцій `Introduction` і `Related Work` для статті
про `ESP32 CL-HAR`. Текст ставить нашу роботу в контекст TinyML HAR,
on-device continual learning, TinyOL/replay, Kwon/LifeLearner, PACL+, COOL і
новіших робіт з on-device training/pruning на MCU.

## Introduction

Human activity recognition (HAR) на основі wearable або embedded IMU-сенсорів є
важливою задачею для health monitoring, fitness tracking, smart environments і
industrial safety. Типовий TinyML workflow для HAR виглядає так: модель
навчається offline на великому dataset, квантується, а потім deployment artifact
прошивається на microcontroller. Такий підхід добре підходить для inference, але
погано реагує на зміни після deployment.

У реальних умовах HAR-система стикається з distribution shift:

- новий користувач;
- інше положення або кріплення сенсора;
- інша амплітуда руху;
- інший IMU hardware;
- зміна поведінки користувача з часом;
- відмінність між public dataset і конкретним embedded deployment.

Повне retraining у cloud або на PC не завжди є прийнятним: воно потребує
передачі даних, збільшує latency, може порушувати privacy, а також не підходить
для пристроїв у field deployment. Тому on-device continual learning є природним
напрямом: пристрій має частково адаптуватися до нових даних після deployment.

Водночас microcontrollers мають жорсткі обмеження. `ESP32-WROOM-32` має лише
`320 KB SRAM` class memory budget і не має зовнішньої PSRAM у поточному target.
Це робить повне on-device навчання CNN практично недоцільним. Натомість більш
реалістичним є split architecture:

```text
offline trained frozen feature extractor
+ lightweight on-device trainable head
+ small replay buffer
```

У цій роботі ми перевіряємо, чи можна реалізувати такий мінімальний
replay-based continual learning pipeline на `ESP32-WROOM-32` у `Rust/no_std`
firmware stack для IMU-HAR:

```text
MPU6050
-> MicroFlow-32 frozen feature extractor
-> OnlineLayer32
-> RAM-only ReplayBuffer32
-> FIFO або reservoir-per-class replay
```

Ми не претендуємо на `state-of-the-art` HAR accuracy або нову складну neural
architecture. Мета роботи інша: надати відтворюваний, ресурсно-прозорий ESP32
baseline для supervised in-session continual learning з реальним IMU-сенсором.

### Contributions

Основні внески роботи:

1. Реалізовано `Rust/no_std` ESP32 pipeline для IMU-HAR з реальним
   `MPU6050`.
2. Підготовлено `MicroFlow-32` frozen feature extractor як практичний embedded
   path і порівняно його з `MicroFlow-64`.
3. Реалізовано `OnlineLayer32` і `ReplayBuffer32` у Rust без C++/Arduino
   runtime.
4. Порівняно `no_adapt`, `FIFO replay` і `reservoir-per-class replay` під
   однаковим RAM budget.
5. Зафіксовано resource metrics: inference latency, online head latency,
   CL update time, replay RAM estimate і firmware footprint.
6. Проведено real-device pilot, який показує prediction shift після supervised
   labels на реальному `MPU6050` сигналі.

## Related Work

### 1. TinyML and on-device learning on microcontrollers

TinyML традиційно зосереджений на deployment квантизованих inference models на
малих microcontrollers. Класична проблема полягає в тому, що після deployment
модель стає статичною: вона може ефективно виконувати inference, але не
адаптується до нових даних або змін середовища.

`TinyOL` запропонував ідею TinyML з online learning на microcontrollers:
модель може інкрементально оновлюватись на streaming data в умовах обмежених
ресурсів. Для нашої роботи TinyOL важливий як алгоритмічна мотивація:
адаптацію треба робити малою, локальною і сумісною з MCU constraints. Наша
система продовжує цю лінію, але фокусується на IMU-HAR, ESP32, реальному
MPU6050 і порівнянні replay policies.

Сучасна робота Fusco et al. `On-device training and pruning for energy saving
and continuous learning in resource-constrained MCUs` також прямо працює з
resource-constrained MCU setting. Вона пропонує on-device pruning під час
incremental onboard training для зменшення latency та енергоспоживання,
експериментує на ESP32 і STM32, і орієнтується на пристрої з дуже малим SRAM
budget. Це близька за hardware/resource мотивацією робота, але її фокус -
pruning/energy trade-off і IIoT dataset, тоді як наш фокус - HAR, real IMU,
RAM-only replay і FIFO/reservoir comparison.

### 2. Continual learning for mobile and embedded sensing

Kwon et al. важливі для цієї роботи з двох причин. По-перше, їхні дослідження
mobile/embedded sensing показують, що continual learning methods треба
оцінювати не лише за accuracy, а й за system performance на sensing workloads.
По-друге, `LifeLearner` демонструє hardware-aware meta continual learning для
embedded computing platforms з latent replay, product quantization і deployment
на edge devices / MCU-class hardware.

`LifeLearner` є сильнішим і складнішим reference: він використовує
hardware-aware meta learning, latent replay і compression techniques. Наша
робота не намагається повторити або перевершити LifeLearner за algorithmic
complexity. Натомість ми беремо простішу позицію: мінімальний frozen extractor
+ trainable head + RAM-only replay baseline, який можна повністю відтворити на
`ESP32-WROOM-32` і прозоро профілювати.

Таким чином Kwon/LifeLearner задає верхній контекст: embedded continual learning
може бути складним і hardware-aware. Наша робота займає нижчий, baseline-рівень:
що можна зробити на звичайному ESP32 без PSRAM, без persistence і без складної
compression pipeline.

### 3. Online continual learning for HAR

Schiemer et al. `Online continual learning for human activity recognition`
формулюють HAR як online continual learning scenario, де sensor data надходить
streaming mode і система має адаптуватися до нових activities або змін
distribution. Ця робота важлива як HAR-specific continual learning background:
вона показує, що real-world HAR не є статичною supervised classification
задачею.

PACL+ (`Online continual learning using proxy-anchor and contrastive loss with
Gaussian replay for sensor-based human activity recognition`) пропонує
складніший HAR continual learning framework, що поєднує Proxy Anchor loss,
contrastive learning і Gaussian replay. PACL+ орієнтований на високу
performance stability і state-of-the-art benchmark results на HAR datasets.
Порівняно з PACL+, наша робота свідомо простіша: ми не вводимо contrastive loss
або Gaussian replay, а перевіряємо, чи працює мінімальний RAM-only replay
baseline на ESP32 з реальним IMU.

Ці HAR-specific роботи показують, що continual learning для HAR є актуальною
задачею. Водночас багато таких підходів залишаються desktop/offline або
архітектурно складнішими, ніж те, що доцільно реалізовувати як перший baseline
на `ESP32-WROOM-32`.

### 4. On-device continual learning for HAR on MCU

Найближчим сучасним reference є `COOL: continual online on-device learning for
human activity recognition enhanced by KANs`. COOL прямо розглядає
HAR + continual online on-device learning і оцінює hardware performance на
`STM32H743`. COOL використовує Kolmogorov-Arnold Networks (KANs) і показує
значні performance improvements для HAR scenarios, а також повідомляє
on-device inference/training time близько `44.30 ms` і `46.03 ms`.

COOL є близьким за постановкою, але відрізняється за цілями і складністю:

- COOL використовує KAN-based classifier;
- hardware target - `STM32H743`, а не `ESP32-WROOM-32`;
- робота заявляє сильніші HAR performance improvements;
- architecture складніша за наш мінімальний replay baseline.

Наша робота не стверджує, що перевершує COOL. Навпаки, COOL використовується як
сильний related reference, відносно якого ми позиціонуємо себе як простіший,
resource-transparent ESP32 baseline:

```text
COOL:
  stronger HAR+CL system, KAN-based, STM32H743, real-time adaptation

This work:
  minimal replay-based CL baseline, ESP32-WROOM-32, Rust/no_std,
  real MPU6050, RAM-only FIFO/reservoir comparison
```

### 5. Positioning of this work

У порівнянні з наведеними роботами наша позиція така:

| Work family | Main focus | Difference from this work |
| --- | --- | --- |
| TinyOL | online learning on MCUs | не HAR-specific ESP32 + real MPU6050 replay comparison |
| Kwon / LifeLearner | hardware-aware meta CL, latent replay, compression | складніший CL stack; наша робота є minimal ESP32 baseline |
| OCL-HAR / PACL+ | HAR continual learning algorithms | переважно benchmark/algorithm focus; не ESP32 Rust/no_std implementation |
| COOL | HAR on-device CL with KANs on STM32H743 | сильніший і складніший MCU HAR+CL reference; не ESP32 minimal replay baseline |
| Fusco et al. pruning | on-device training/pruning and energy saving on MCUs | pruning/energy/IIoT focus; не IMU-HAR FIFO/reservoir replay |

Наша робота заповнює практичну нішу між TinyML inference і складними HAR+CL
systems:

```text
ESP32-WROOM-32
+ real MPU6050
+ Rust/no_std firmware
+ frozen MicroFlow-32 feature extractor
+ OnlineLayer32
+ RAM-only ReplayBuffer32
+ FIFO vs reservoir-per-class
+ resource metrics
+ pilot prediction shift
```

Тобто внесок роботи не в новій складній CL-методиці, а у відтворюваній
embedded реалізації і resource-oriented evaluation мінімального replay-based CL
pipeline на ESP32-class MCU.

## Reference Notes

Це робочі citation notes для подальшого оформлення bibliography:

- Ren, Anicic, Runkler. `TinyOL: TinyML with Online-Learning on Microcontrollers`.
  IJCNN 2021. DOI: `10.1109/IJCNN52387.2021.9533927`.
- Kwon et al. `LifeLearner: Hardware-Aware Meta Continual Learning System for
  Embedded Computing Platforms`. SenSys 2023. DOI: `10.1145/3625687.3625804`.
- Schiemer, Fang, Dobson, Ye. `Online continual learning for human activity
  recognition`. Pervasive and Mobile Computing, 2023. DOI:
  `10.1016/j.pmcj.2023.101817`.
- Mittal et al. `PACL+: Online continual learning using proxy-anchor and
  contrastive loss with Gaussian replay for sensor-based human activity
  recognition`. Expert Systems with Applications, 2025. DOI:
  `10.1016/j.eswa.2025.128603`.
- Fusco et al. `On-device training and pruning for energy saving and continuous
  learning in resource-constrained MCUs`. Future Generation Computer Systems,
  2026. DOI: `10.1016/j.future.2025.108194`.
- Liu et al. `COOL: continual online on-device learning for human activity
  recognition enhanced by KANs`. CCF Transactions on Pervasive Computing and
  Interaction, 2026. DOI: `10.1007/s42486-026-00229-z`.

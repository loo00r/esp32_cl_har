# Development Log

Покрокова документація розробки системи Continual Learning для HAR на ESP32 (Rust).
Цей файл ведеться паралельно з розробкою і слугуватиме основою для розділу **Implementation** у статті.

---

## Фаза 0 — C++ прототип (PlatformIO)

**Мета**: переконатись що плата працює, порт доступний, базові операції (Blink, Serial) функціонують.

**Середовище**: PlatformIO + Arduino framework, C++

**Результат**:
- Плата: ESP32-D0WD-V3 rev3.1, 240 MHz, 4 MB Flash, `/dev/ttyUSB0`
- LED на GPIO2 блимає, Serial monitor працює
- Прототип зберігається в окремому репозиторії як reference

---

## Фаза 1b — Перехід на Rust toolchain

**Мета**: налаштувати Rust-середовище для Xtensa ESP32, запустити першу прошивку.

### Крок 1: Встановлення espup

```bash
cargo install espup
espup install
```

`espup` встановлює:
- Xtensa LLVM backend (clang для ESP32 ISA)
- `xtensa-esp-elf-gcc` — лінкер для bare-metal таргету
- Rust toolchain `esp` (форк компілятора з підтримкою Xtensa)

Після встановлення додали до `~/.bashrc`:
```bash
echo '. $HOME/export-esp.sh' >> ~/.bashrc
. $HOME/export-esp.sh  # для поточного сеансу
```

### Крок 2: Встановлення espflash

```bash
cargo install espflash
```

`espflash` — утиліта для прошивки ESP32 через USB. Налаштована як runner у `.cargo/config.toml`:
```toml
[target.xtensa-esp32-none-elf]
runner = "espflash flash --monitor --chip esp32"
```

### Крок 3: Генерація проекту

Проект згенеровано через офіційний шаблон `esp-rs/esp-template`:
- Target: `xtensa-esp32-none-elf`
- HAL: `esp-hal v1.0` (офіційний, `no_std`)
- Без RTOS (bare-metal)

Ключові файли конфігурації:
- `rust-toolchain.toml` — фіксує toolchain `esp`
- `.cargo/config.toml` — таргет, runner, rustflags
- `build.rs` — додає linker script `linkall.x`, friendly error messages

### Крок 4: Перша прошивка

```bash
. $HOME/export-esp.sh
cargo run
```

Вивід монітора підтвердив успішне завантаження:
```
Chip type:    esp32 (revision v3.1)
Crystal frequency: 40 MHz
Flash size:   4MB
Features:     WiFi, BT, Dual Core, 240MHz
Flashing has completed!
ESP-IDF v5.5.1 2nd stage bootloader
boot: Loaded app from partition at offset 0x10000
```

**Що зроблено**: базовий loop з затримкою 500 мс, без виводу і периферії.

### Крок 5: Blink — перший GPIO

Використано `esp_hal::gpio::Output` для керування GPIO2 (вбудований LED):

```rust
use esp_hal::gpio::{Level, Output, OutputConfig};

let mut led = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());

loop {
    led.toggle();
    // busy-wait 500ms
}
```

**Примітка**: в `esp-hal v1.0` `Output::new` приймає 3 аргументи — пін, початковий рівень, `OutputConfig`. У попередніх версіях HAL був інший API (2 аргументи).

**Результат**: LED GPIO2 блимає з інтервалом 500 мс. Перша взаємодія з периферією підтверджена.

---

## Наступні кроки

- [ ] Hello World через UART (`esp-println`)
- [ ] MicroFlow inference: вбудувати `model.tflite`, тестовий forward pass
- [ ] Тренування 1D-CNN на WISDM (Python/TensorFlow)

---

## Нотатки по архітектурі

### Чому `no_std`

ESP32 має 320 KB SRAM. Rust std потребує аллокатора та OS-абстракцій (threads, filesystem), які на bare-metal відсутні. `no_std` дає повний контроль над пам'яттю без overhead.

### Чому esp-hal, а не esp-idf-hal

| | esp-hal (no_std) | esp-idf-hal (std) |
|---|---|---|
| Підтримка | Офіційна Espressif | Community |
| Overhead | Мінімальний | ESP-IDF OS під капотом |
| Підходить для | Inference + CL loop | Загальне IoT |

Для real-time inference та on-device навчання критична мінімальна латентність — обрано `esp-hal`.

### Структура пам'яті (орієнтовно)

| Ресурс | Загально | Зарезервовано | Доступно |
|--------|----------|---------------|---------|
| SRAM | 320 KB | ~30 KB (stack, HAL) | ~290 KB |
| Flash | 4 MB | ~83 KB (bootloader) | ~3.9 MB |

---

## Фаза 0c — Повернення mainline на Rust

**Що зроблено**: відкотили останній коміт, який переводив репозиторій на `C++/PlatformIO`, через `git revert`, тому mainline знову містить `Cargo.toml`, Rust toolchain конфігурацію і `src/bin/main.rs`. Також вирівняли документацію під Rust-first напрям і додали `AGENTS.md` як актуальний контракт для агентів.

**Рішення**: використано `git revert`, а не `git reset`, щоб не зачепити локальні незакомічені зміни в робочому дереві.

---

## Фаза 0d — Відновлення деталізації плану

**Що зроблено**: після повернення на Rust переглянули `PLAN.md` і `CLAUDE.md` та повернули туди втрачені дослідницькі деталі: related work, повні фази, flow системи, метрики, експерименти, розміри replay buffer і правила роботи. Усі описи переведено на Rust-first архітектуру без повернення до C++ runtime.

**Рішення**: замість короткого "переписаного з нуля" плану збережено попередню глибину документації, але змінено технологічний базис на `Rust/no_std`.

---

## Фаза 0e — Фіксація thesis-рамки

**Що зроблено**: додано окремий файл `THESIS.md`, який фіксує вузьку дослідницьку рамку майбутньої статті без зміни `PLAN.md`. У файл винесено core thesis, експериментальний дизайн `no adaptation` vs `FIFO` vs `reservoir-per-class`, межі MVP і список того, що свідомо лишається поза поточним scope.

**Рішення**: thesis винесено в окремий артефакт, щоб не перевантажувати `PLAN.md` статейними формулюваннями і водночас мати короткий reference проти scope drift під час реалізації.

---

## Фаза 0f — Очищення commit scope

**Що зроблено**: оновлено `.gitignore`, щоб локальні артефакти `.pio/` і `.codex` не потрапляли в робочий commit scope. Також окремо зафіксовано, що `AGENTS.md` і локальні AI-контекстні файли не повинні комітитись у mainline.

**Рішення**: перед початком реалізації звузили commit scope до файлів, що реально рухають проєкт або документацію статті, і відсікли локальне середовище та допоміжний шум.

---

## Фаза 0g — Ігнорування зовнішнього HAR reference repo

**Що зроблено**: додано `Deep-Learning-for-Human-Activity-Recognition/` у `.gitignore`. Цей каталог використовується як локальний reference для training pipeline, але не входить до основного репозиторію firmware.

**Рішення**: зовнішній код лишається доступним для адаптації ідей та скриптів, але не забруднює історію mainline як вкладений сторонній репозиторій.

---

## Фаза 1c — MPU6050 smoke test over I2C

**Що зроблено**: додано мінімальний Rust-модуль `src/mpu6050.rs` для роботи з `MPU6050` через `esp-hal` I2C master без зовнішніх драйверних crate. У firmware `src/bin/main.rs` замість blink-only циклу піднято `I2C0` на `100 kHz`, виконано probe адрес `0x68` і `0x69`, читання `WHO_AM_I`, wake-up через `PWR_MGMT_1`, а далі циклічне логування сирих `accel/gyro` значень у serial monitor.

**Рішення**: для першого апаратного тесту не додавали повноцінний abstraction layer або фільтрацію. Ціль цього кроку — швидко підтвердити, що wiring `GPIO21/GPIO22`, адреса сенсора і базове читання регістрів працюють на реальній платі, залишаючи API простим для наступного кроку з sensor sampling path.

---

## Фаза 1d — Підготовка 20 Hz акселерометричного sampling loop

**Що зроблено**: firmware переведено з разового debug-читання на окремий `20 Hz` loop для акселерометра. У `src/mpu6050.rs` додано легший шлях `read_accel()` без читання `gyro/temp`, а в `src/bin/main.rs` введено фіксований період `50 ms`, scheduler через `Instant`, і обмежене логування раз на секунду, щоб serial output менше впливав на cadence.

**Рішення**: для `Фази 1` важливіше стабільно зчитувати `ax/ay/az` з правильним темпом, ніж постійно друкувати весь сирий пакет `14` байт. Тому лог зменшено, а sampling cadence зроблено явним у коді. Фактичне апаратне підтвердження цього кроку лишається окремою перевіркою на платі.

**Виміри**: під час реального запуску на платі отримано рівномірні логи `samples=20, 40, 60...` з `t_ms≈1004, 2004, 3004...`, що підтверджує стабільний темп близько `20 Hz`. Типовий час одного циклу читання становив `~868 us`, без `mpu6050 read error` у видимому інтервалі тесту.

---

## Фаза 2a — Підготовка WISDM preprocessing pipeline у notebook

**Що зроблено**: у `notebooks/CNN_training.ipynb` зібрано базовий preprocessing pipeline для `WISDM` під майбутню `1D-CNN` модель. Після початкового EDA дані переведено в окремий `df_model`: колонки нормалізовано до `user_id/x/y/z`, сирі значення приведено до числових типів, записи відсортовано по `user_id/timestamp`, а активності зіставлено з числовими `label`.

**Що зроблено далі**: виконано глобальний `z-score` для трьох осей акселерометра з формуванням окремого `df_model_z`, щоб не затирати попередній етап. Перевірка показала `mean≈0` і `std≈1` для `x/y/z` після стандартизації.

**Windowing**: дані сегментовано у вікна `80x3` з overlap `50%` (`step=40`) окремо в межах кожної пари `user_id + activity`, без змішування різних суб'єктів і класів в одному сегменті.

**Проміжний результат**:
- сирий очищений датасет після bootstrap preprocessing: `1,086,465` рядків
- кількість суб'єктів: `36`
- сформовано `26,893` вікон форми `(80, 3)`
- масиви:
  - `X.shape = (26893, 80, 3)`
  - `y.shape = (26893,)`
  - `subjects.shape = (26893,)`
  - `y_onehot.shape = (26893, 6)`
- перевірка цілісності:
  - `NaN in X = 0`
  - `Inf in X = 0`
  - `NaN in y = 0`

**Рішення**: на цьому кроці свідомо зупинилися перед побудовою моделі. Preprocessing і segmentation зафіксовано як окремий завершений підетап, щоб далі окремо реалізувати `Conv1D` архітектуру і `Leave-One-Subject-Out CV` без змішування етапів.

---

## Фаза 2b — Фіксація baseline `Conv1D` архітектури

**Що зроблено**: у `notebooks/CNN_training.ipynb` додано і перевірено базову `1D-CNN` архітектуру для вікон `80x3`:

- `Conv1D(32, kernel_size=5, activation='relu', padding='same')`
- `Conv1D(64, kernel_size=3, activation='relu', padding='same')`
- `GlobalAveragePooling1D()`
- `Dense(6, activation='softmax')`

Модель успішно зібрана в TensorFlow/Keras і дала такий summary:

- `input shape`: `(80, 3)`
- після першого `Conv1D`: `(80, 32)`
- після другого `Conv1D`: `(80, 64)`
- після `GlobalAveragePooling1D`: `64` ознаки
- вихід: `6` класів
- `trainable params`: `7,110` (`~27.77 KB` у float32-представленні)

**Чому саме така архітектура**:

- `ReLU` обрано як просту і практичну нелінійність для TinyML / embedded-friendly baseline: вона дешева, стандартна і краще узгоджується з подальшою `INT8` квантизацією, ніж складніші активації на кшталт `GELU`.
- `Softmax` у фінальному шарі використано тому, що задача є `single-label` класифікацією з `6` взаємовиключними активностями; потрібен нормалізований розподіл імовірностей по класах, а не незалежні `sigmoid`-оцінки.
- `GlobalAveragePooling1D` обрано замість `Flatten`, щоб уникнути зайвого росту кількості параметрів перед класифікатором і тримати baseline компактним для подальшого embedded deployment.

**Рішення**: на цьому етапі свідомо не ускладнювали модель `GELU`, attention-блоками, LSTM або глибшими head-частинами. Мета поточного baseline — не максимальна desktop-`accuracy`, а легка, зрозуміла і потенційно квантизовна архітектура, від якої далі можна перейти до `LOSO CV`, `INT8` export і embedded inference path.

---

## Фаза 2c — Перехід на fold-aware preprocessing без leakage

**Що зроблено**: у `notebooks/CNN_training.ipynb` переписано preprocessing-частину так, щоб вона відповідала майбутньому `Leave-One-Subject-Out CV`, а не глобальному preprocessing по всьому датасету. Замість одноразового `z-score` на всьому `df_model` і глобального windowing тепер введено окремі helper-функції:

- `prepare_dataframe(...)`
- `add_contiguous_segments(...)`
- `fit_zscore_stats(...)`
- `apply_zscore_stats(...)`
- `create_windows_from_segments(...)`
- `build_one_fold_data(...)`

**Що змінено по суті**:

- прибрано leakage від глобальної нормалізації: `mean/std` тепер мають рахуватись тільки на train-subjects всередині одного fold;
- тестовий subject нормалізується тими ж train-статистиками;
- windowing тепер йде не по всьому `user_id + activity`, а по `user_id + activity + segment_id`;
- `segment_id` вводиться через аналіз розривів у `timestamp`, щоб вікна не проходили через штучно склеєні часові шматки одного й того ж класу.

**Рішення**: це не косметичне прибирання, а критичне виправлення experimental protocol. Без цієї правки `single-fold` і майбутній `LOSO CV` були б методично слабшими через optimistic leakage і некоректне windowing через часові розриви.

**Примітка**: після зміни логіки очищено outputs у змінених notebook-cells, тому актуальні значення треба повторно прогнати в Jupyter вже по новому fold-aware pipeline.

---

## Фаза 2d — Baseline `LOSO CV` для `1D-CNN`

**Що зроблено**: після переходу на fold-aware preprocessing виконано повний `Leave-One-Subject-Out CV` для baseline `1D-CNN` моделі на `WISDM`. Для кожного `test_subject_id` окремо будувалися:

- train/test split по суб'єктах
- `z-score` статистики тільки на train-subjects
- windowing `80x3` з overlap `50%`
- новий екземпляр baseline-моделі

**Архітектура baseline**:

- `Conv1D(32, kernel_size=5, activation='relu', padding='same')`
- `Conv1D(64, kernel_size=3, activation='relu', padding='same')`
- `GlobalAveragePooling1D()`
- `Dense(6, activation='softmax')`

**Підсумкові метрики `LOSO CV`**:

- `mean accuracy = 0.8130`
- `std accuracy = 0.1096`
- `accuracy = 0.8217`
- `macro avg f1-score = 0.7809`
- `weighted avg f1-score = 0.8193`

**Per-class результати**:

- `Walking`: precision `0.8732`, recall `0.9142`, f1 `0.8933`
- `Jogging`: precision `0.9333`, recall `0.9123`, f1 `0.9227`
- `Upstairs`: precision `0.5940`, recall `0.5871`, f1 `0.5905`
- `Downstairs`: precision `0.5209`, recall `0.4672`, f1 `0.4926`
- `Sitting`: precision `0.9638`, recall `0.7389`, f1 `0.8365`
- `Standing`: precision `0.9486`, recall `0.9512`, f1 `0.9499`

**Спостереження**:

- baseline добре відрізняє `Walking`, `Jogging`, `Standing`
- найбільша плутанина спостерігається між `Walking`, `Upstairs` і `Downstairs`
- розкид `accuracy` по fold-ах помітний, що підтримує thesis про user-level domain shift і потребу в подальшій on-device адаптації

**Рішення**: на цьому етапі не продовжували offline-tuning архітектури. Отриманий `LOSO` baseline уже достатньо сильний, щоб перейти до наступного кроку `Фази 2`: підготовки `representative dataset`, `INT8 quantization` і експорту `model.tflite`.

---

## Фаза 2e — `INT8` quantization і експорт `model.tflite`

**Що зроблено**: після завершення `LOSO CV` baseline-модель натреновано на фінальному наборі `X_final`, побудованому з усього доступного корпусу вікон `80x3` після `z-score` нормалізації. Для quantization не використовували один довільний `LOSO` fold, а сформували окремий фінальний training corpus і на його основі зібрали representative dataset.

**Representative dataset**:

- фінальний корпус: `X_final.shape = (9154, 80, 3)`
- `y_final.shape = (9154,)`
- representative set побудовано як class-balanced піднабір
- по `32` вікна на кожен з `6` класів
- підсумковий representative set: `X_representative.shape = (192, 80, 3)`

**Рішення по representative data**: для `INT8` calibration навмисно не використовували windows лише з одного subject або одного `LOSO` fold. Representative set повинен відбивати загальну train-distribution входів, а не частковий split. Додатково введено балансування по класах, щоб calibration не перекошувалась у бік домінуючих класів `Walking/Jogging`.

**Експорт**:

- baseline-модель експортовано через `TFLiteConverter`
- застосовано `tf.lite.Optimize.DEFAULT`
- використано `representative_dataset`
- `supported_ops = TFLITE_BUILTINS_INT8`
- `inference_input_type = int8`
- `inference_output_type = int8`

**Збережені артефакти**:

- `model.tflite`: `/tmp/esp32_cl_har_artifacts/baseline_cnn_int8.tflite`
- metadata: `/tmp/esp32_cl_har_artifacts/baseline_cnn_int8_metadata.json`

**Підсумкові quantization параметри**:

- input shape: `[1, 80, 3]`
- output shape: `[1, 6]`
- input dtype: `int8`
- output dtype: `int8`
- `input_scale = 0.030599215999245644`
- `input_zero_point = 9`
- `output_scale = 0.00390625`
- `output_zero_point = -128`

**Що це означає**: `Фаза 2` практично закрита. Тепер існує повний baseline artifact для embedded inference path. Водночас для `Фази 4` з `OnlineLayer 64 -> 6` імовірно знадобиться окремий export feature extractor варіанту, який віддає `64`-вимірний feature vector, а не фінальні `6` logits/classes.

---

## Фаза 0h — Уточнення thesis-positioning проти новіших робіт

**Що зроблено**: після перегляду новіших статей `2025–2026` уточнено дослідницьке позиціонування в `THESIS.md` і `PLAN.md`. Явно зафіксовано, що найближчим прямим аналогом є `COOL (2026)`, тоді як `PACL+ (2025)`, `TrustTiny-HAR (2026)` і новіші TinyML HAR роботи використовуються як сильний фон для `Related Work`, replay-мотивації та resource-oriented comparison.

**Рішення**: не роздували документацію окремими research memo файлами. Натомість мінімально і прямо скоригували thesis-рамку: робота не претендує на `state-of-the-art` accuracy, а захищає відтворюваний `ESP32 + Rust/no_std + MPU6050` replay-based baseline з порівнянням `FIFO` vs `reservoir-per-class` під жорстким memory budget.

---

## Фаза 3.0a — Підготовка MicroFlow-friendly export path

**Що зроблено**: перед стартом embedded inference path перевірили сумісність quantized graph із `MicroFlow` як primary Rust-first runtime кандидатом. Початковий `Conv1D + GlobalAveragePooling1D` export давав небажані `EXPAND_DIMS` і `MEAN`, тому в notebook зібрано окремий `MicroFlow`-friendly варіант через `Conv2D + AveragePooling2D`.

**Нові артефакти pipeline**:

- baseline classifier для `MicroFlow`: `80x3x1 -> 6`
- feature extractor для CL pipeline: `80x3x1 -> 64`

**Проміжний результат**:

- classifier artifact: `/tmp/esp32_cl_har_artifacts/microflow_classifier_int8.tflite`
- feature extractor artifact: `/tmp/esp32_cl_har_artifacts/microflow_feature_extractor_int8.tflite`
- classifier output shape: `[1, 6]`
- feature extractor output shape: `[1, 64]`

**Виявлений blocker**: після першого `MicroFlow`-friendly export graph усе ще містив службові ops `SHAPE`, `STRIDED_SLICE` і `PACK` поруч із `RESHAPE`. Найімовірніше це походить від явного `Reshape((64,))` у шарі `feature_vector`.

**Мінімальна правка**: notebook оновлено так, щоб `feature_vector` будувався через `Flatten`, а не через явний `Reshape((64,))`. Це локальна технічна правка без зміни загальної архітектури, потрібна лише для очищення TFLite graph перед повторним compatibility check.

**Статус**: compatibility gate ще не закрито остаточно. Наступний крок — повторити training/export для `MicroFlow`-friendly моделі й ще раз перевірити список ops. Лише після clean graph можна переходити до Rust integration у `Фазі 3`.

**Оновлення після повторної перевірки**: `Flatten` не прибрав службові ops `SHAPE`, `STRIDED_SLICE`, `PACK` і `RESHAPE`. Тому `MicroFlow`-гілку далі спрощено до full-conv варіанту без переходу `4D tensor -> vector`: classifier head тепер планується як `Conv2D(6, 1x1) + Softmax`, а feature extractor віддає `1x1x64` tensor. Це не змінює baseline path і не зачіпає вже завершену `Фазу 2`, а лише додає окремий deployment-oriented export path для `Фази 3.0`.

**Фінальний результат compatibility gate**: full-conv export path дав clean `MicroFlow`-friendly op graphs без службових `SHAPE/STRIDED_SLICE/PACK/RESHAPE`.

- classifier artifact: `/tmp/esp32_cl_har_artifacts/microflow_fullconv_classifier_int8.tflite`
- classifier input: `[1, 80, 3, 1]`
- classifier output: `[1, 1, 1, 6]`
- classifier ops: `CONV_2D -> CONV_2D -> AVERAGE_POOL_2D -> CONV_2D -> SOFTMAX`

- feature extractor artifact: `/tmp/esp32_cl_har_artifacts/microflow_fullconv_feature_extractor_int8.tflite`
- feature extractor input: `[1, 80, 3, 1]`
- feature extractor output: `[1, 1, 1, 64]`
- feature extractor ops: `CONV_2D -> CONV_2D -> AVERAGE_POOL_2D`

**Висновок**: `Фаза 3.0` пройдена успішно. Для подальшої Rust integration у `Фазі 3` використовуємо саме full-conv `MicroFlow`-compatible artifacts, а не старий `Conv1D/GAP` export. Старі baseline artifacts залишаються валідними для offline baseline і статейного comparison, але не є основним deployment path для `MicroFlow`.

---

## Фаза 3a — Rust skeleton для inference path без runtime backend

**Що зроблено**: у firmware додано мінімальний `no_std` skeleton для `Фази 3`, не змінюючи базову логіку зчитування `MPU6050` і не переходячи ще до реального inference runtime.

**Додані модулі**:

- `src/model.rs`
  - shape-константи для `80x3x1` input path
  - розміри classifier / feature extractor outputs
  - назви artifact-ів `microflow_fullconv_*`
- `src/window.rs`
  - ring-buffer `SlidingWindow` на `80` семплів
  - доступ до впорядкованих semplів без heap allocation
- `src/quant.rs`
  - `z-score` статистики
  - `input_scale / input_zero_point`
  - quantization input window
  - dequantization feature tensor
- `src/inference.rs`
  - `MicroflowStub`
  - простий API `classify()` і `extract_features()`
  - явний `BackendUnavailable`, поки реального runtime ще немає

**Що змінено в `main.rs`**:

- додано `SlidingWindow`
- додано stride-логіку `80` / `40`
- після заповнення вікна формується quantized input tensor
- викликається classifier stub і feature extractor stub
- якщо backend ще не інтегрований, firmware явно логує, що спрацював skeleton path

**Що це дає**:

- `main.rs` уже знає про model artifacts, input shape і CL-oriented feature path
- сенсорний loop більше не ізольований від майбутнього inference path
- наступний крок тепер добре локалізований: замінити `MicroflowStub` на реальний runtime wrapper без повторного переписування sampling/window/quantization частини

**Що ще не зроблено**:

- реальний `MicroFlow` backend не інтегровано
- `.tflite` артефакти ще не вбудовані у firmware
- реальний forward pass ще не виконується
- `cargo run` / hardware test не запускали

---

## Фаза 3b — Узгодження sensor scale з WISDM preprocessing

**Що зроблено**: у quantization path виправлено критичний scale mismatch між сирими `MPU6050` значеннями і статистиками `WISDM`. Драйвер `mpu6050.rs` залишився low-level і продовжує повертати raw `i16` counts, але в [`src/quant.rs`](/home/g00n3r/projects/esp32_cl_har/src/quant.rs:1) перед `z-score` тепер виконується явна конверсія `raw -> m/s²`.

**Прийняте припущення**:

- після reset `MPU6050` працює в default accel range `±2g`
- для цього режиму використано `16384 LSB/g`
- конверсія:
  - `g = raw / 16384.0`
  - `m/s² = g * 9.80665`

**Чому це важливо**: `WISDM` preprocessing і нормалізація в notebook працювали в фізичних одиницях, а не в raw ADC counts. Без цієї правки firmware формально міг би робити inference, але подавав би моделі неправильний input distribution.

**Що це дає**:

- `quantize_window()` тепер ближче відтворює той самий preprocessing path, що й offline model pipeline
- sensor driver залишається простим і перевіряємим
- conversion logic локалізована в quantization module, де їй і місце

**Що ще треба перевірити далі**:

- host-side sanity check layout/quantization проти Python
- лише після цього підключати реальний `MicroFlow` backend

---

## Фаза 3c — Host-side sanity check для quantization/layout

**Що зроблено**: додано окремий host-side script [`scripts/quant_sanity_check.py`](/home/g00n3r/projects/esp32_cl_har/scripts/quant_sanity_check.py:1), який перевіряє, що Rust-side quantization path відтворює Python reference для full-conv `MicroFlow` classifier artifact.

**Що саме перевіряє script**:

- бере реальний contiguous `80`-sample window з локального `WISDM`
- використовує metadata від `microflow_fullconv_classifier_int8.tflite`
- формує Python reference quantized tensor
- окремо симулює Rust path:
  - `m/s² -> raw MPU6050 counts`
  - `raw -> m/s²`
  - `z-score`
  - `int8 quantization`
- порівнює плоский input tensor layout довжини `240`

**Результат**:

- input shape: `[1, 80, 3, 1]`
- flat tensor length: `240`
- `max_abs_diff = 0`
- `mismatch_count = 0`

**Висновок**: поточний Rust path для `SlidingWindow -> quantize_window()` узгоджений із Python/TFLite preprocessing як по scale, так і по flat layout. Це знімає найбільш небезпечний ризик перед інтеграцією реального inference backend: що модель отримувала б правильний graph, але неправильний input distribution або переплутаний tensor order.

---

## Фаза 3d — Рішення по inference runtime без оверінженірингу

**Що вирішено**: після чистого проходження `MicroFlow` compatibility gate ми свідомо не зробили `MicroFlow` core deployment path. Причина не в graph-ах, а в тому, що compile-time/API quirks почали зміщувати фокус роботи з CL на боротьбу з inference бібліотекою.

**Прийняте практичне рішення**:

- `MicroFlow` лишається evaluated Rust-first candidate
- frozen inference stage для `ESP32` рухається через practical `TFLite Micro / esp-tflite-micro`-compatible backend
- Rust/no_std scope концентрується там, де лежить реальна новизна роботи:
  - `SlidingWindow`
  - preprocessing / scale alignment
  - `OnlineLayer`
  - `ReplayBuffer`
  - `FIFO vs reservoir-per-class`
  - firmware orchestration і resource profiling

**Чому це відповідає thesis**: архітектурна ідея Kwon/LifeLearner зберігається. Ми так само тримаємо `frozen / quantized feature extractor` і окремий lightweight trainable classifier, але не намагаємось зробити inference runtime новизною статті. Наша новизна — мінімалістична ESP32-адаптація CL split-model під жорсткіші обмеження, а не pure-Rust CNN inference будь-якою ціною.

---

## Фаза 3e — Повернення firmware до practical backend boundary

**Що зроблено**: після рішення не робити `MicroFlow` blocker-ом для `Фази 3` код повернуто до чистого і компільованого backend-boundary state.

**Зміни**:

- з `Cargo.toml` прибрано залежність `microflow`
- [`src/inference.rs`](/home/g00n3r/projects/esp32_cl_har/src/inference.rs:1) спрощено до `FrozenInferenceBackend` stub
- `main.rs` більше не містить half-integrated `MicroFlow` wrapper logic
- full-conv `.tflite` artifacts залишено в репозиторії як deployment-ready assets для подальшої practical integration через `TFLite Micro / esp-tflite-micro`-compatible backend

**Що це дає**:

- firmware знову не залежить від нестабільного compile-time API конкретної inference бібліотеки
- `SlidingWindow`, preprocessing, quantization, feature path і backend boundary лишаються на місці
- далі можна інтегрувати practical frozen inference backend без повторного переписування sensor loop і без втрати вже зробленого research pipeline

**Перевірка**:

- виконано лише `cargo build`
- результат: compile success
- `cargo run` і hardware flashing не запускались

---

## Фаза 4a — Ізольований Rust `OnlineLayer`

**Що зроблено**: розпочато `Фазу 4` не через інтеграцію в `main.rs`, а через ізольований trainable head модуль у чистому Rust. Це свідомо не закриває `Фазу 3` і не чіпає frozen inference backend, а готує ту частину системи, яка і є ядром нашої новизни.

**Додано**:

- [`src/online_layer.rs`](/home/g00n3r/projects/esp32_cl_har/src/online_layer.rs:1)
  - `OnlineLayer { weights[[64][6]], bias[6] }`
  - `forward_logits()`
  - `forward()` з локальною `softmax`
  - `backward_batch()` для mini-batch update
- експорт модуля через [`src/lib.rs`](/home/g00n3r/projects/esp32_cl_har/src/lib.rs:1)

**Що це дає**:

- CL trainable head більше не залежить від frozen inference backend
- логіку `64 -> 6` уже можна перевіряти і розвивати окремо
- наступним кроком можна робити `ReplayBuffer` таким самим ізольованим шляхом, без змішування з runtime integration

**Перевірка**:

- виконано лише `cargo build`
- результат: compile success
- `cargo run` і hardware flashing не запускались

**Примітка**: модуль поки не інтегровано в основний loop. Це саме `Фаза 4a`, а не повна `Фаза 4`.

---

## Фаза 4a.2 — Embedded smoke test binary для `OnlineLayer`

**Що зроблено**: перед переходом до `ReplayBuffer` додано окремий embedded smoke-test binary для перевірки `OnlineLayer` вже в MCU-контексті, але без frozen inference backend, без сенсора і без будь-яких runtime flash writes.

**Додано**:

- [`src/bin/online_layer_smoke.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/online_layer_smoke.rs:1)
  - hardcoded synthetic `features[64]`
  - `OnlineLayer.forward()`
  - `OnlineLayer.backward_batch()`
  - логування `forward` / `backward` latency
  - перевірка, що вихід після update змінюється
  - простий LED heartbeat

**Навіщо це потрібно**:

- не переходити одразу до `ReplayBuffer` і повної `Фази 4`, не перевіривши, що `f32`-логіка trainable head взагалі стабільно збирається і готова до першого hardware smoke test
- відокремити CL head verification від frozen inference backend decision
- зберегти порядок маленьких перевіряємих кроків під `ESP32-WROOM-32`

**Перевірка**:

- виконано лише `cargo build`
- результат: compile success
- `cargo run`, flashing і runtime-перевірка на підключеній платі поки не виконувались

**Примітка**: це саме підготовка до `embedded smoke test`, а не повна інтеграція `OnlineLayer` у firmware loop.

---

## Фаза 4a.3 — Embedded smoke test `OnlineLayer` на ESP32

**Що зроблено**: виконано перший реальний hardware smoke test для [`src/bin/online_layer_smoke.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/online_layer_smoke.rs:1) на підключеній `ESP32-WROOM-32`.

**Що перевірено**:

- `cargo run --bin online_layer_smoke` успішно прошив плату через `espflash`
- boot пройшов штатно
- firmware почав серійне логування без `panic`, `reset` або watchdog-симптомів
- `OnlineLayer.forward()` і `OnlineLayer.backward_batch()` реально виконуються на MCU

**Спостереження з логів**:

- типові `forward` latency: близько `94–95 us`
- типові `backward_batch` latency: близько `243–247 us`
- вихід після `backward_batch()` змінювався між ітераціями, тобто update path реально працює
- LED heartbeat також працював, тобто main loop лишався живим
- при довгій серії однакових synthetic update-ів спостерігався дрейф prediction, тому numerical stability trainable head ще треба окремо звірити перед переходом до `ReplayBuffer`

**Що це означає**:

- `f32`-логіка trainable head для `64 -> 6` уже не лише компілюється, а й реально виконується на `ESP32`
- можна переходити до наступного ізольованого CL-модуля, не боячись, що сам `OnlineLayer` принципово не підходить для MCU
- це все ще не інтеграція з frozen feature extractor і не повна `Фаза 4`, а саме підтвердження життєздатності CL head на залізі

**Примітка**: smoke test використовував synthetic `features[64]`, без сенсора, без inference backend і без runtime flash writes. Це був навмисно мінімальний hardware checkpoint перед `ReplayBuffer`.

---

## Фаза 4a.4 — Мінімальна стабілізація `OnlineLayer` на ESP32

**Що зроблено**: після першого hardware smoke test внесено одну локальну стабілізаційну правку без зміни архітектури CL head.

**Зміни**:

- у [`src/online_layer.rs`](/home/g00n3r/projects/esp32_cl_har/src/online_layer.rs:1) саморобний `exp_approx()` замінено на `libm::expf()` для більш передбачуваної `softmax`
- у [`src/bin/online_layer_smoke.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/online_layer_smoke.rs:1) learning rate для smoke test знижено до `0.01`
- у [`Cargo.toml`](/home/g00n3r/projects/esp32_cl_har/Cargo.toml:1) додано залежність `libm`

**Перевірка**:

- виконано `cargo build`
- після цього повторно виконано `cargo run --bin online_layer_smoke` на реальній `ESP32`

**Спостереження з логів**:

- steady-state `forward` latency: близько `46–51 us`
- steady-state `backward_batch` latency: близько `174–179 us`
- ймовірність цільового класу `p0` зростала монотонно:
  - приблизно від `0.1667` на старті
  - до `0.89+` у довшій серії update-ів
- prediction більше не дрейфував у бік іншого класу під тим самим synthetic batch

**Що це означає**:

- ізольований Rust `OnlineLayer` тепер не лише виконується на `ESP32`, а й демонструє чисельно стабільніший update path
- це достатній stopping point для `Фази 4a`
- наступним кроком уже можна повертатись до планового рішення: не лізти одразу в повну `Фазу 4`, а повернутись до `Фази 3` practical frozen inference backend

---

## Фаза 3f — Bounded feasibility gate для `MicroFlow` як frozen feature extractor backend

**Що зроблено**: виконано окрему bounded feasibility перевірку `MicroFlow` не як ядра проєкту, а лише як можливого Rust-only backend для frozen feature extractor. Головну research-архітектуру не змінювали.

**Перевірений model path**:

- старий baseline artifact лишився доступним поза git:
  - `/tmp/esp32_cl_har_artifacts/baseline_cnn_int8.tflite`
- у репозиторії використано вже підготовлений full-conv deployment variant:
  - [`src/model_artifacts/microflow_fullconv_classifier_int8.tflite`](/home/g00n3r/projects/esp32_cl_har/src/model_artifacts/microflow_fullconv_classifier_int8.tflite:1)
  - [`src/model_artifacts/microflow_fullconv_feature_extractor_int8.tflite`](/home/g00n3r/projects/esp32_cl_har/src/model_artifacts/microflow_fullconv_feature_extractor_int8.tflite:1)
- для bounded gate обрано саме feature extractor `80x3x1 -> 1x1x1x64`, без replay, без CL loop, без змін в основному `main.rs`

**Перевірка TFLite ops**:

- `microflow_fullconv_classifier_int8.tflite`:
  - `CONV_2D`
  - `CONV_2D`
  - `AVERAGE_POOL_2D`
  - `CONV_2D`
  - `SOFTMAX`
- `microflow_fullconv_feature_extractor_int8.tflite`:
  - `CONV_2D`
  - `CONV_2D`
  - `AVERAGE_POOL_2D`
- старий `baseline_cnn_int8.tflite` для порівняння все ще містить несумісні з `MicroFlow` службові ops:
  - `EXPAND_DIMS`
  - `RESHAPE`
  - `MEAN`

**Висновок по graph compatibility**:

- full-conv feature extractor graph чистий і підходить для bounded `MicroFlow`-перевірки
- переписувати notebook/model pipeline далі не потрібно

**API finding по `MicroFlow`**:

локальний source `microflow-macros` показав точний generated contract:

- `predict(input: Buffer4D<f32, ...>) -> Buffer4D<f32, ...>`
- `predict_quantized(input: Buffer4D<i8, ...>) -> Buffer4D<f32, ...>`
- всередині generated code:
  - `predict()` робить `Tensor4D::quantize(...)`
  - `predict_quantized()` створює `Tensor4D::new(...)`
  - далі `predict_inner(...)`
  - результат завжди повертається через `.dequantize()`

**Що це означає practically**:

- public API `MicroFlow` для нашого кейсу не є raw-`int8`-only
- `f32` на межі API — це очікувана поведінка бібліотеки, а не ознака поломки quantized path
- quantized inference все одно виконується всередині runtime

**Кодова ізоляція**:

- у [`Cargo.toml`](/home/g00n3r/projects/esp32_cl_har/Cargo.toml:1) додано optional feature:
  - `microflow_backend`
- `MicroFlow` не став обов'язковою залежністю default build
- додано окремий модуль [`src/inference_microflow.rs`](/home/g00n3r/projects/esp32_cl_har/src/inference_microflow.rs:1)
  - `MicroflowFeatureBackend`
  - `extract_features(&[f32; 240]) -> [f32; 64]`
- додано окремий prepared smoke binary [`src/bin/microflow_feature_smoke.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/microflow_feature_smoke.rs:1)
  - synthetic normalized input
  - один feature extraction pass
  - checksum / first features / latency logging
  - не інтегровано в `main.rs`

**Build checks**:

- `cargo build --offline` -> success
- `cargo build --release --features microflow_backend --bin microflow_feature_smoke --offline` -> success

**Resource / size findings**:

- `microflow_feature_smoke` (`release`):
  - `text = 41657`
  - `data = 532`
  - `bss = 176`
  - `dec = 238265`
  - `.rodata = 10448`
- для порівняння `frozen_artifact_smoke` (`release`):
  - `text = 55365`
  - `data = 604`
  - `bss = 176`
  - `dec = 251973`
  - `.rodata = 29676`

**Інтерпретація**:

- bounded `MicroFlow` candidate компілюється під наш `xtensa-esp32-none-elf` target
- default path не ламається
- явного `std` blocker на build-рівні нема
- але dependency surface суттєво важчий, ніж здається з одного `microflow` crate:
  - `microflow`
  - `microflow-macros`
  - `nalgebra`
  - `simba`
  - `flatbuffers` та супутній proc-macro stack
- generated code використовує `nalgebra` напряму, тому для навіть ізольованого candidate path довелося додати `nalgebra` як optional direct dependency

**Hardware smoke test**:

- виконано `cargo run --features microflow_backend --bin microflow_feature_smoke`
- плата: `ESP32-WROOM-32`, chip revision `v3.1`, flash `4MB`
- app / partition size з `espflash`: `112,896 / 4,128,768 bytes`, тобто `2.73%`
- boot пройшов штатно
- `MicroFlow` feature extractor виконав один inference pass без `panic`, reset або watchdog symptoms

**Runtime log**:

- `backend=microflow-fullconv-feature-extractor`
- `latency_us=298204`
- `checksum=61.496418`
- first features:
  - `f0=3.5906632`
  - `f1=0.05057272`
  - `f2=0.5562999`
  - `f3=0.40458176`

**Оновлене рішення після hardware smoke test**:

- `MicroFlow` більше не `uncertain` на рівні feasibility
- clean ops, build success, acceptable static footprint і one-shot runtime на реальній ESP32 підтверджені
- класифікація зараз:
  - **A) MicroFlow feasible candidate**

**Обмеження рішення**:

- це ще не повна інтеграція в основний `main.rs`
- це ще не streaming sensor loop
- це ще не перевірка latency на реальному MPU6050 window
- dependency/API surface все ще складніший, ніж у власного простого Rust-модуля, тому `MicroFlow` лишається backend candidate, а не scientific contribution

**Рекомендація**:

- наступний practical крок: підключити `MicroFlowFeatureBackend` до існуючого `FrozenInferenceBackend` boundary або окремого Phase 3 path, але без зміни CL-архітектури
- `TFLite Micro / esp-tflite-micro` лишається fallback, якщо streaming integration або resource profiling покаже проблему

---

## Фаза 3g — `MicroFlow` INT8 input smoke test

**Проблема**: попередній hardware smoke test використовував `MicroFlowFeatureExtractor::predict(...)` з public API `f32[240]`. Це підтверджувало роботу generated backend, але лишало відкрите питання, чи можна тримати production-like pipeline однорідним:

```text
MPU6050 raw -> m/s² -> WISDM z-score -> INT8 input tensor -> MicroFlow
```

**API уточнення**: local macro expansion `target/microflow-expansion.rs` показав два generated шляхи:

- `predict(f32)`:
  - приймає `Buffer4D<f32, 1, 80, 3, 1>`
  - всередині викликає `Tensor4D::quantize(input, input_scale, input_zero_point)`
  - виконує INT8 graph
  - повертає dequantized `Buffer4D<f32, 1, 1, 1, 64>`
- `predict_quantized(i8)`:
  - приймає вже готовий `Buffer4D<i8, 1, 80, 3, 1>`
  - створює quantized tensor через `Tensor4D::new(...)`
  - виконує той самий INT8 graph
  - повертає dequantized `Buffer4D<f32, 1, 1, 1, 64>`

**Що зроблено**:

- у [`src/inference_microflow.rs`](/home/g00n3r/projects/esp32_cl_har/src/inference_microflow.rs:1) додано окремий quantized path:
  - `MicroflowQuantizedInput = Buffer4D<i8, 1, 80, 3, 1>`
  - `make_quantized_input(...)`
  - `extract_features_quantized(&[i8; 240]) -> [f32; 64]`
- у [`Cargo.toml`](/home/g00n3r/projects/esp32_cl_har/Cargo.toml:11) додано окремий smoke binary:
  - `microflow_feature_quantized_smoke`
- додано [`src/bin/microflow_feature_quantized_smoke.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/microflow_feature_quantized_smoke.rs:1):
  - synthetic normalized input
  - explicit quantization через `quantize_scalar(..., INPUT_SCALE, INPUT_ZERO_POINT)`
  - один `predict_quantized(i8)` feature extraction pass
  - latency/checksum/first features logging
  - без sensor loop, без CL, без replay, без NVS/persistence

**Команди**:

```bash
. $HOME/export-esp.sh && cargo build --features microflow_backend --bin microflow_feature_quantized_smoke
. $HOME/export-esp.sh && cargo run --features microflow_backend --bin microflow_feature_quantized_smoke
```

Перший non-TTY `cargo run` успішно прошив firmware, але monitor не зміг стартувати через `Failed to initialize input reader`. Повтор у TTY-режимі дав serial logs.

**Build / flash result**:

- build: success
- target: `xtensa-esp32-none-elf`
- chip: `ESP32 rev v3.1`
- flash: `4MB`
- app / partition size: `112,496 / 4,128,768 bytes`, тобто `2.72%`

**Runtime log**:

- `backend=microflow-fullconv-feature-extractor`
- `input_shape=[1,80,3,1]`
- `input_dtype=i8`
- `output_shape=[1,1,1,64]`
- `latency_us=297631`
- `checksum=61.496418`
- first features:
  - `f0=3.5906632`
  - `f1=0.05057272`
  - `f2=0.5562999`
  - `f3=0.40458176`

**Висновок**:

- `MicroFlowFeatureExtractor::predict_quantized(i8)` працює на реальній `ESP32-WROOM-32`
- результат збігається з попереднім `f32` smoke test на тому самому synthetic input після quantization
- для production-like Phase 3 path можна використовувати однорідний `INT8` вхід:

```text
normalized window -> i8[240] -> MicroFlow predict_quantized -> f32[64] features
```

**Рішення**:

- основний embedded inference flow має йти через `predict_quantized(i8)`, не через public `f32` input
- `predict(f32)` лишається тільки diagnostic convenience path
- `MicroFlow` після цього кроку лишається feasible primary Rust-only candidate для frozen feature extractor; `TFLite Micro / esp-tflite-micro` лишається fallback, а не наступним обов'язковим кроком

---

## Фаза 3h — Housekeeping після INT8 smoke test

**Що зроблено**: після quick refactor промарковано активні й неактивні smoke paths, щоб код не створював враження кількох рівнозначних production-flow.

**Кодове маркування**:

- [`src/bin/microflow_feature_quantized_smoke.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/microflow_feature_quantized_smoke.rs:1) позначено як активний `Phase 3` smoke для цільового flow `i8[240] -> MicroFlow predict_quantized() -> f32[64]`
- [`src/bin/microflow_feature_smoke.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/microflow_feature_smoke.rs:1) позначено як diagnostic-only перевірку public `f32` API
- [`src/bin/frozen_artifact_smoke.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/frozen_artifact_smoke.rs:1) позначено як archived checkpoint для перевірки read-only model artifacts
- [`src/bin/online_layer_smoke.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/online_layer_smoke.rs:1) позначено як archived `Phase 4a` checkpoint для synthetic `OnlineLayer`
- [`src/inference.rs`](/home/g00n3r/projects/esp32_cl_har/src/inference.rs:1) позначено як тимчасовий default stub boundary для основного sensor loop
- [`src/inference_microflow.rs`](/home/g00n3r/projects/esp32_cl_har/src/inference_microflow.rs:1) уточнено: `extract_features(...)` є diagnostic `f32` path, а `extract_features_quantized(...)` є цільовим Phase 3 path

**Перевірка**:

```bash
cargo build
cargo build --features microflow_backend --bin microflow_feature_quantized_smoke
```

Обидві команди виконались успішно.

**Рішення**: старі smoke binaries не видалялися, бо вони є корисними reproducibility checkpoints для DEVLOG/статті. Вони лише явно відмічені як diagnostic або archived, а активний embedded inference напрям зараз один: `predict_quantized(i8)`.

---

## Фаза 3i — Streaming `MicroFlow` feature extraction у `main.rs`

**Що зроблено**: підключено вже перевірений `MicroFlowFeatureBackend::extract_features_quantized(...)` в основний sensor loop за feature flag `microflow_backend`. Default build без feature лишається на `FrozenInferenceBackend` stub, щоб не робити `MicroFlow` обов'язковою залежністю.

**Активний flow з feature flag**:

```text
MPU6050 raw accel
-> SlidingWindow 80 samples
-> stride 40 samples
-> quantize_window(...)
-> i8[240]
-> MicroFlow predict_quantized(...)
-> f32[64] feature tensor
```

**Межі кроку**:

- без classifier head
- без `OnlineLayer`
- без replay
- без UART labels
- без NVS / persistence / runtime flash writes

**Команди**:

```bash
cargo build
cargo build --features microflow_backend --bin esp32_cl_har
. $HOME/export-esp.sh && cargo run --features microflow_backend --bin esp32_cl_har
```

**Build / flash result**:

- default `cargo build`: success
- `microflow_backend` build: success
- hardware run: success
- chip: `ESP32 rev v3.1`
- flash: `4MB`
- app / partition size: `125,376 / 4,128,768 bytes`, тобто `3.04%`

**Runtime observations**:

- MPU6050 detected:
  - address `0x68`
  - `WHO_AM_I=0x70`
- windowing спрацював після `80` samples
- inference запускався кожні `40` samples
- приклади runtime logs:
  - `attempt=1`, `inference_us=299094`, `input_q0=2`, `feat0=0`, `feat1=0`, `feat2=0.10114544`, `feat3=0.5057272`
  - `attempt=2`, `inference_us=297881`, `input_q0=-11`, `feat0=0`, `feat1=0`, `feat2=0.20229088`, `feat3=0.6068726`
  - steady-state attempts далі трималися близько `297710 us`

**Висновок**:

- `MicroFlow` уже працює не лише як isolated smoke test, а й у реальному streaming sensor path
- Phase 3 довела базову відтворюваність frozen feature extractor на ESP32
- feature tensor `f32[64]` доступний для наступного підключення `OnlineLayer`

**Важливе обмеження**:

- поточний synchronous inference займає приблизно `298 ms`
- це блокує `20 Hz` sampling на кожному inference stride і видно по часових логах після inference
- це не ламає feasibility result, але означає, що перед фінальною CL-інтеграцією треба прийняти timing рішення:
  - або лишити як чесний resource-limited ESP32 baseline
  - або зменшити feature extractor
  - або змінити scheduling / sampling policy

**Рішення**: не додавати CL поверх цього одразу. Наступний крок має бути короткий Phase 3 timing/resource checkpoint: зафіксувати latency impact і вирішити, чи лишаємо `64` features, чи робимо легшу `32`-feature model variant.

---

## Фаза 3j — Streaming latency statistics для `MicroFlow-64`

**Що зроблено**: додано lightweight latency accumulator у `main.rs` тільки для `microflow_backend` path:

- `min_us`
- `mean_us`
- `max_us`
- summary кожні `10` inference attempts

Додаткових heap allocation, replay, CL, persistence або flash writes не додавалося.

**Умови тесту**:

- плата і MPU6050 лежали нерухомо
- це правильно для latency/resource checkpoint, бо на цьому кроці вимірюється runtime, а не якість activity recognition
- movement tests потрібні пізніше для domain-shift / qualitative sensor validation

**Команди**:

```bash
cargo build
cargo build --features microflow_backend --bin esp32_cl_har
. $HOME/export-esp.sh && cargo run --features microflow_backend --bin esp32_cl_har
```

**Build / flash result**:

- default build: success
- `microflow_backend` build: success
- app / partition size: `125,840 / 4,128,768 bytes`, тобто `3.05%`

**Runtime latency result**:

Після `10` attempts:

- `min_us=298624`
- `mean_us=298742`
- `max_us=299802`

Після `20` attempts:

- `min_us=298620`
- `mean_us=298683`
- `max_us=299802`

**Висновок**:

- `MicroFlow-64` latency дуже стабільна: приблизно `298.7 ms` на feature extraction
- jitter малий, але absolute latency велика для strict `20 Hz` безперервного sampling
- inference блокує приблизно `6` sampling periods по `50 ms`

**Рішення**:

- `MicroFlow-64` лишається working Rust-only baseline extractor
- перед підключенням `OnlineLayer` треба зробити hardware-aware decision:
  - або чесно лишити `64` features як resource-limited baseline
  - або підготувати `32`-feature variant і порівняти latency / accuracy / replay RAM

---

## Фаза 3k — Notebook для `MicroFlow-32` ablation

**Що зроблено**: створено окрему копію training notebook для перевірки легшого `32`-feature extractor без зміни основного `64`-feature baseline notebook.

**Новий файл**:

- [`notebooks/CNN_training_microflow32.ipynb`](/home/g00n3r/projects/esp32_cl_har/notebooks/CNN_training_microflow32.ipynb:1)

**Зміни відносно `CNN_training.ipynb`**:

- outputs очищені, щоб не змішувати старі `64`-feature результати з новим ablation run
- додано `MICROFLOW_FEATURE_DIM = 32`
- у MicroFlow full-conv моделі другий `Conv2D` тепер використовує `filters=feature_dim`
- feature extractor output має стати `1x1x1x32`
- export names змінені, щоб не перезаписати `64` artifacts:
  - `microflow_fullconv32_classifier_int8.tflite`
  - `microflow_fullconv32_feature_extractor_int8.tflite`
- metadata доповнено:
  - `microflow_feature_dim`
  - `ablation = "microflow32"`

**Що не змінювали**:

- preprocessing
- LOSO/fold-aware logic
- representative dataset logic
- quantization flow
- MicroFlow-friendly full-conv op pattern

**Як запускати**:

Запускати notebook зверху вниз локально. Очікувані артефакти після export:

```text
/tmp/esp32_cl_har_artifacts/microflow_fullconv32_classifier_int8.tflite
/tmp/esp32_cl_har_artifacts/microflow_fullconv32_classifier_int8_metadata.json
/tmp/esp32_cl_har_artifacts/microflow_fullconv32_feature_extractor_int8.tflite
/tmp/esp32_cl_har_artifacts/microflow_fullconv32_feature_extractor_int8_metadata.json
```

**Рішення**: `32`-feature path готується як ablation, а не як заміна `64` baseline. Після запуску треба порівняти `64` vs `32` за latency, Flash footprint, offline accuracy і replay RAM.

---

## Фаза 3l — Результати notebook run для `MicroFlow-32`

**Що перевірено**: переглянуто outputs у [`notebooks/CNN_training_microflow32.ipynb`](/home/g00n3r/projects/esp32_cl_har/notebooks/CNN_training_microflow32.ipynb:1) після локального запуску notebook.

**MicroFlow-32 architecture**:

- input: `(80, 3, 1)`
- `Conv2D(32, kernel=(5,3))`
- `Conv2D(32, kernel=(3,1))`
- `AveragePooling2D(pool=(74,1))`
- classifier head: `Conv2D(6, kernel=(1,1)) + Softmax`
- feature extractor output: `(1, 1, 1, 32)`

**Parameter count**:

- classifier: `3,814` params (`~14.90 KB` float32 representation)
- feature extractor: `3,616` params (`~14.12 KB` float32 representation)
- для порівняння попередній `MicroFlow-64` classifier мав `7,110` params, feature extractor `6,720` params

**Training output у notebook**:

- final MicroFlow-32 training epoch:
  - `accuracy=0.8713`
  - `loss=0.3512`
  - `val_accuracy=0.8483`
  - `val_loss=0.5206`
- feature sample:
  - shape: `(1, 1, 1, 32)`
  - dtype: `float32`
  - first values: `[1.0827651, 4.4670916, 0.04870715, 0.7682283, ...]`

**Artifact paths**:

```text
/tmp/esp32_cl_har_artifacts/microflow_fullconv32_classifier_int8.tflite
/tmp/esp32_cl_har_artifacts/microflow_fullconv32_classifier_int8_metadata.json
/tmp/esp32_cl_har_artifacts/microflow_fullconv32_feature_extractor_int8.tflite
/tmp/esp32_cl_har_artifacts/microflow_fullconv32_feature_extractor_int8_metadata.json
```

**Artifact sizes**:

- classifier `.tflite`: `9.4 KB`
- feature extractor `.tflite`: `8.1 KB`

**Quantization metadata**:

- input shape: `[1, 80, 3, 1]`
- input dtype: `int8`
- input scale: `0.030599215999245644`
- input zero point: `9`
- classifier output shape: `[1, 1, 1, 6]`
- classifier output scale: `0.00390625`
- classifier output zero point: `-128`
- feature output shape: `[1, 1, 1, 32]`
- feature output scale: `0.07324092090129852`
- feature output zero point: `-128`
- representative samples: `192`

**TFLite ops**:

Classifier:

```text
CONV_2D
CONV_2D
AVERAGE_POOL_2D
CONV_2D
SOFTMAX
```

Feature extractor:

```text
CONV_2D
CONV_2D
AVERAGE_POOL_2D
```

**Висновок**:

- `MicroFlow-32` artifact clean з точки зору ops і сумісний з тим самим bounded MicroFlow path
- output shape і metadata відповідають очікуваному `32`-feature extractor
- PC-side validation accuracy не просіла критично відносно `64` path, тому `32`-feature extractor є реальним кандидатом для ESP32 latency ablation

**Наступний крок**:

- скопіювати `microflow_fullconv32_feature_extractor_int8.tflite` і metadata у `src/model_artifacts/`
- додати окремий `MicroFlow-32` backend/feature flag або тимчасово перемкнути artifact для виміру latency
- заміряти streaming latency на ESP32 і порівняти з `MicroFlow-64` (`mean ≈ 298.7 ms`)

---

## Фаза 3m — ESP32 latency ablation для `MicroFlow-32`

**Що зроблено**:

- скопійовано `MicroFlow-32` feature extractor artifact у firmware artifacts:
  - [`src/model_artifacts/microflow_fullconv32_feature_extractor_int8.tflite`](/home/g00n3r/projects/esp32_cl_har/src/model_artifacts/microflow_fullconv32_feature_extractor_int8.tflite:1)
  - [`src/model_artifacts/microflow_fullconv32_feature_extractor_int8_metadata.json`](/home/g00n3r/projects/esp32_cl_har/src/model_artifacts/microflow_fullconv32_feature_extractor_int8_metadata.json:1)
- у `.gitignore` додано вузький виняток `!src/model_artifacts/*.tflite`, бо ці малі deployment artifacts потрібні для `include_bytes!` і відтворюваного firmware build
- додано feature flag `microflow32_backend`
- додано окремий backend module:
  - [`src/inference_microflow32.rs`](/home/g00n3r/projects/esp32_cl_har/src/inference_microflow32.rs:1)
- `main.rs` тепер підтримує три режими:
  - default stub backend
  - `microflow_backend` для `64` features
  - `microflow32_backend` для `32` features

**Команди**:

```bash
cargo build
cargo build --features microflow_backend --bin esp32_cl_har
cargo build --features microflow32_backend --bin esp32_cl_har
. $HOME/export-esp.sh && cargo run --features microflow32_backend --bin esp32_cl_har
```

**Build / flash result для `MicroFlow-32`**:

- default build: success
- `MicroFlow-64` build: success
- `MicroFlow-32` build: success
- hardware run: success
- app / partition size: `124,784 / 4,128,768 bytes`, тобто `3.02%`

**Runtime latency result для `MicroFlow-32`**:

Після `10` attempts:

- `min_us=171961`
- `mean_us=172072`
- `max_us=173058`

Після `20` attempts:

- `min_us=171960`
- `mean_us=172017`
- `max_us=173058`

**Порівняння `64` vs `32`**:

| Extractor | Features | Mean latency | App size | Feature RAM | Replay RAM (`6 x 16 x dim x f32`) |
|---|---:|---:|---:|---:|---:|
| `MicroFlow-64` | `64` | `~298.7 ms` | `125,840 bytes` | `256 B` | `24 KB` |
| `MicroFlow-32` | `32` | `~172.0 ms` | `124,784 bytes` | `128 B` | `12 KB` |

**Interpretation**:

- `MicroFlow-32` зменшив streaming feature extraction latency приблизно на `42%`
- replay memory для майбутнього `FIFO/reservoir` CL зменшується вдвічі
- firmware Flash size майже не змінився, бо більша частина footprint іде від runtime/generated support code, а не тільки від кількості latent features
- notebook validation accuracy для `32` path не просіла критично, тому `32` features виглядають кращим embedded candidate

**Рішення**:

- `MicroFlow-64` лишається stronger baseline/reference
- `MicroFlow-32` стає практичним основним кандидатом для подальшого ESP32 CL path
- наступний крок перед `OnlineLayer`: зафіксувати feature-dim як compile-time параметр або підготувати `OnlineLayer32`, не змішуючи `32` і `64` шляхи в одному runtime

---

## Фаза 3n — PC TFLite vs ESP MicroFlow-32 consistency

**Що зроблено**: перевірено, що `MicroFlow-32` на ESP32 рахує той самий exported `.tflite` artifact, що й Python/TensorFlow Lite interpreter на PC. GPU не використовувався; це CPU-side TFLite reference check.

**Input**:

- deterministic `int8[240]`
- той самий synthetic pattern, що в попередніх smoke tests:

```text
normalized[i] = i * 0.0125 - 1.0
quantized[i] = round(normalized[i] / input_scale) + input_zero_point
```

- input scale: `0.030599215999245644`
- input zero point: `9`
- first input values:

```text
[-24, -23, -23, -22, -22, -22, -21, -21]
```

**PC reference command**:

```bash
/home/g00n3r/.venvs/base/bin/python - <<'PY'
# TensorFlow Lite Interpreter over src/model_artifacts/microflow_fullconv32_feature_extractor_int8.tflite
PY
```

**PC TFLite result**:

- model: `src/model_artifacts/microflow_fullconv32_feature_extractor_int8.tflite`
- output shape: `[1, 1, 1, 32]`
- output dtype: `int8`
- output quantization:
  - scale: `0.07324092090129852`
  - zero point: `-128`
- output quantized first8:

```text
[-127, -106, -71, -112, -95, -98, -126, -119]
```

- dequantized checksum:

```text
45.55585479736328
```

- dequantized first8:

```text
[0.07324092, 1.61130023, 4.17473269, 1.17185473,
 2.41695046, 2.19722772, 0.14648184, 0.65916830]
```

**ESP32 command**:

```bash
. $HOME/export-esp.sh && cargo run --features microflow32_backend --bin microflow32_consistency_smoke
```

**ESP32 MicroFlow result**:

- app / partition size: `111,488 / 4,128,768 bytes`, тобто `2.70%`
- backend: `microflow-fullconv32-feature-extractor`
- output shape: `[1, 1, 1, 32]`
- latency: `173037 us`
- checksum:

```text
45.55585
```

- first8:

```text
[0.07324092, 1.6113002, 4.1747327, 1.1718547,
 2.4169505, 2.1972277, 0.14648184, 0.6591683]
```

**Висновок**:

- PC TFLite і ESP MicroFlow-32 outputs збігаються в межах очікуваного float formatting
- це підтверджує, що ESP32 рахує саме exported TFLite artifact, а не просто повертає довільні features
- `MicroFlow-32` можна використовувати як основний frozen feature extractor candidate для наступного `OnlineLayer` integration

---

## Фаза 3o — MPU6050 vs WISDM domain-shift probe

**Що зроблено**: додано і запущено окремий probe binary для первинної оцінки domain shift між реальним `MPU6050` і статистиками `WISDM`, на яких тренувався preprocessing/model pipeline.

**Новий binary**:

- [`src/bin/mpu_domain_shift_probe.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/mpu_domain_shift_probe.rs:1)

**Що вимірює**:

- raw accelerometer LSB stats
- converted `m/s²` stats
- `z-score` stats відносно `WISDM_ZSCORE_STATS`
- quantized `int8` min/max і saturation counts

**Межі кроку**:

- без inference
- без `OnlineLayer`
- без replay
- без persistence / NVS / flash writes
- сенсор лежав нерухомо; це baseline stationary distribution, а не activity-recognition test

**Команди**:

```bash
cargo build --bin mpu_domain_shift_probe
. $HOME/export-esp.sh && cargo run --bin mpu_domain_shift_probe
```

**Build / flash result**:

- build: success
- hardware run: success
- app / partition size: `116,368 / 4,128,768 bytes`, тобто `2.82%`
- samples: `200`
- sampling rate: `20 Hz`

**WISDM normalization constants**:

```text
means = [0.664113, 7.246045, 0.397697]
stds  = [6.876277, 6.739789, 4.761111]
input_scale = 0.030599216
input_zero_point = 9
```

**Stationary MPU6050 result**:

Axis 0:

```text
raw mean=-1246.54, std=31.78, min=-1356, max=-1168
mps2 mean=-0.7461, std=0.0190
zscore mean=-0.2051, std=0.0028
quant_i8 min=2, max=3, sat_min=0, sat_max=0
```

Axis 1:

```text
raw mean=187.46, std=37.71, min=96, max=280
mps2 mean=0.1122, std=0.0226
zscore mean=-1.0585, std=0.0034
quant_i8 min=-26, max=-25, sat_min=0, sat_max=0
```

Axis 2:

```text
raw mean=16032.24, std=55.94, min=15904, max=16172
mps2 mean=9.5961, std=0.0323
zscore mean=1.9320, std=0.0071
quant_i8 min=72, max=73, sat_min=0, sat_max=0
```

**Висновок**:

- stationary MPU6050 distribution суттєво відрізняється від WISDM normalization center, особливо на `axis1` і `axis2`
- gravity домінує на `axis2`, що очікувано для нерухомого сенсора
- quantization не saturates, тому input scale поки придатний для hardware path
- цей результат треба використати в Discussion як обґрунтування domain shift і потреби continual adaptation

**Рішення**:

- Фаза 3 тепер має functional inference, latency/resource trade-off, PC-vs-ESP consistency і базовий domain-shift evidence
- наступний practical крок: перейти до `OnlineLayer32` forward-only integration, без replay/training/persistence

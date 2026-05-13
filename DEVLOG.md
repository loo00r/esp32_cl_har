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

## Фаза 3p — RAM / firmware size checkpoint для MicroFlow-32 і MicroFlow-64

**Що зроблено**: закрито RAM/resource checkpoint для Phase 3 на рівні build sections, firmware footprint і ручної оцінки контрольованих буферів. Hardware inference/stability для `MicroFlow-32` і `MicroFlow-64` уже перевірялися раніше; на цьому кроці `cargo run` не потрібен.

**Команди**:

```bash
cargo build --features microflow32_backend --bin esp32_cl_har
xtensa-esp32-elf-size -A target/xtensa-esp32-none-elf/debug/esp32_cl_har
xtensa-esp32-elf-size target/xtensa-esp32-none-elf/debug/esp32_cl_har

cargo build --features microflow_backend --bin esp32_cl_har
xtensa-esp32-elf-size -A target/xtensa-esp32-none-elf/debug/esp32_cl_har
xtensa-esp32-elf-size target/xtensa-esp32-none-elf/debug/esp32_cl_har

cargo build --release --features microflow32_backend --bin esp32_cl_har
xtensa-esp32-elf-size -A target/xtensa-esp32-none-elf/release/esp32_cl_har
xtensa-esp32-elf-size target/xtensa-esp32-none-elf/release/esp32_cl_har

cargo build --release --features microflow_backend --bin esp32_cl_har
xtensa-esp32-elf-size -A target/xtensa-esp32-none-elf/release/esp32_cl_har
xtensa-esp32-elf-size target/xtensa-esp32-none-elf/release/esp32_cl_har
```

**Debug build results**:

| Build | `.data` | `.bss` | `.data + .bss` | `.rodata` | `.text` | summary `text/data/bss` |
|---|---:|---:|---:|---:|---:|---:|
| `microflow32_backend` | `6,172 B` | `200 B` | `6,372 B` | `43,064 B` | `59,166 B` | `106,746 / 6,172 / 190,436` |
| `microflow_backend` | `6,172 B` | `200 B` | `6,372 B` | `46,296 B` | `60,274 B` | `111,086 / 6,172 / 190,436` |

**Release build results**:

| Build | `.data` | `.bss` | `.data + .bss` | `.rodata` | `.text` | summary `text/data/bss` |
|---|---:|---:|---:|---:|---:|---:|
| `microflow32_backend` | `728 B` | `184 B` | `912 B` | `9,208 B` | `36,969 B` | `51,149 / 728 / 195,880` |
| `microflow_backend` | `728 B` | `184 B` | `912 B` | `12,432 B` | `38,569 B` | `55,973 / 728 / 195,880` |

**Interpretation**:

- `.data + .bss` дає корисну оцінку статичної DRAM, але `size` summary включає `.stack` у `bss`, тому summary `bss=195,880 B` не треба читати як фактично зайняті model buffers
- `.rodata` містить read-only model bytes / constants і є flash-mapped footprint, а не великий SRAM replay buffer
- `.stack` показує зарезервований linker region, а не виміряний high-water mark
- точний peak stack/high-water mark поки не інструментовано; runtime-stability already checked через streaming runs без reset/panic

**Контрольовані RAM буфери поточного Phase 3 path**:

| Component | Size |
|---|---:|
| `SlidingWindow` | `80 x 3 x i16 = 480 B` |
| quantized input tensor | `80 x 3 x i8 = 240 B` |
| `MicroFlow-32` feature output | `32 x f32 = 128 B` |
| `MicroFlow-64` feature output | `64 x f32 = 256 B` |
| future `OnlineLayer32` weights + bias | `(32 x 6 + 6) x f32 = 792 B` |
| future replay buffer for `32` features | `6 x 16 x 32 x f32 = 12,288 B` |
| future replay buffer for `64` features | `6 x 16 x 64 x f32 = 24,576 B` |

**Висновок**:

- Phase 3 RAM/Flash gate не показує blocker для `MicroFlow-32`
- `MicroFlow-32` лишається основним embedded candidate: нижча latency, менший feature/replay RAM, така сама практична accuracy за notebook run
- `MicroFlow-64` лишається reference path, але не основний ESP32 CL candidate
- Фаза 3 вважається закритою для поточного scope: frozen extractor працює на ESP32, latency/resource/consistency/domain-shift зафіксовано

**Наступний крок**:

- перейти до `OnlineLayer32` forward-only integration поверх `MicroFlow-32` features
- не додавати replay, SGD update, UART labels або persistence до окремого forward-only checkpoint

## Фаза 4a.5 — `OnlineLayer32` forward-only integration поверх `MicroFlow-32`

**Що зроблено**: продовжено вже наявну `OnlineLayer` роботу не через новий модуль, а через узгодження з основним embedded path. Після Phase 3 основним frozen extractor candidate став `MicroFlow-32`, тому `OnlineLayer` переведено з hardcoded `64` features на const-generic dimension і додано alias-и:

```rust
OnlineLayer64 = OnlineLayer<64>
OnlineLayer32 = OnlineLayer<32>
```

**Зміни в коді**:

- [`src/online_layer.rs`](/home/g00n3r/projects/esp32_cl_har/src/online_layer.rs:1)
  - `OnlineLayer<const D: usize>`
  - `forward_logits()`, `forward()`, `backward_batch()` тепер працюють з `[f32; D]`
  - додано `OnlineLayer32` для `MicroFlow-32` features
- [`src/bin/online_layer_smoke.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/online_layer_smoke.rs:1)
  - smoke test явно лишено як archived `OnlineLayer64` synthetic checkpoint
- [`src/bin/main.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/main.rs:1)
  - під `microflow32_backend` додано forward-only path:

```text
MPU6050
-> SlidingWindow 80x3
-> quantize_window()
-> MicroFlow-32 feature extractor
-> OnlineLayer32.forward()
-> class/confidence log
```

**Межі кроку**:

- без replay
- без SGD/update у main loop
- без UART labels
- без persistence/NVS/flash writes
- `OnlineLayer32` поки zero-initialized, тому `confidence=0.16666667` є очікуваним uniform sanity output, не activity-recognition accuracy

**Команди**:

```bash
cargo build --bin esp32_cl_har
cargo build --features microflow32_backend --bin esp32_cl_har
cargo build --bin online_layer_smoke
. $HOME/export-esp.sh && cargo run --features microflow32_backend --bin esp32_cl_har
```

**Build result**:

- default firmware build: success
- `microflow32_backend` firmware build: success
- archived `online_layer_smoke` build: success
- `cargo test --lib` не використовується як валідний check для цього bare-metal target: `xtensa-esp32-none-elf` не має стандартного `test` crate/panic handler setup

**Hardware run result**:

- flash: success
- app / partition size: `126,480 / 4,128,768 bytes`, тобто `3.06%`
- `MicroFlow-32` inference continued to run in streaming path
- observed feature extraction latency: approximately `172.9–173.9 ms`
- `OnlineLayer32.forward()` latency:
  - first observed call: `110 us`
  - steady calls: approximately `49–51 us`

Example log:

```text
microflow feature ok: attempt=1, inference_us=173947, input_q0=2, feat0=0, feat1=0.07324092, feat2=5.493069, feat3=0.36620462
online32 forward ok: attempt=1, online_us=110, pred=0(Walking), confidence=0.16666667
microflow feature ok: attempt=2, inference_us=172873, input_q0=2, feat0=0, feat1=0.07324092, feat2=5.493069, feat3=0.36620462
online32 forward ok: attempt=2, online_us=50, pred=0(Walking), confidence=0.16666667
```

**Висновок**:

- `MicroFlow-32 -> OnlineLayer32.forward()` стикується і працює на реальній ESP32
- forward-only `OnlineLayer32` cost малий порівняно з frozen extractor latency
- цей крок не доводить HAR accuracy, бо head ще не має pretrained/adapted weights

**Наступний крок**:

- підготувати pretrained `OnlineLayer32` weights з `MicroFlow-32` classifier head або окремий offline-trained `32 -> 6` head
- тільки після цього оцінювати meaningful class/confidence logs
- replay/SGD/UART/persistence не додавати до завершення pretrained-head checkpoint

## Фаза 4a.6 — Pretrained `OnlineLayer32` head з `MicroFlow-32` classifier artifact

**Що зроблено**: `OnlineLayer32` перестав бути zero-initialized sanity head. Ваги і bias відновлено з quantized `1x1 Conv2D` шару `classifier_head` у `microflow_fullconv32_classifier_int8.tflite` і перенесено в Rust як f32-equivalent `32 -> 6` head.

Це не нове навчання і не зміна архітектури. Це розділення вже навченого full-conv classifier artifact на:

```text
MicroFlow-32 frozen feature extractor: 80x3x1 -> 1x1x1x32
Rust OnlineLayer32 pretrained head:   32 -> 6
```

**Джерело weights**:

- classifier artifact: [`src/model_artifacts/microflow_fullconv32_classifier_int8.tflite`](/home/g00n3r/projects/esp32_cl_har/src/model_artifacts/microflow_fullconv32_classifier_int8.tflite)
- metadata: [`src/model_artifacts/microflow_fullconv32_classifier_int8_metadata.json`](/home/g00n3r/projects/esp32_cl_har/src/model_artifacts/microflow_fullconv32_classifier_int8_metadata.json)
- head tensor у TFLite:
  - weights tensor shape: `[6, 1, 1, 32]`
  - bias tensor shape: `[6]`
  - feature input tensor scale: `0.07324092090129852`

**Зміни в коді**:

- [`src/online_layer.rs`](/home/g00n3r/projects/esp32_cl_har/src/online_layer.rs:1)
  - додано `MICROFLOW32_HEAD_WEIGHTS`
  - додано `MICROFLOW32_HEAD_BIAS`
  - додано `OnlineLayer32::new_microflow32_pretrained()`
- [`src/bin/main.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/main.rs:1)
  - main streaming path тепер створює `OnlineLayer32::new_microflow32_pretrained()`
- [`src/model.rs`](/home/g00n3r/projects/esp32_cl_har/src/model.rs:1)
  - додано `MICROFLOW32_CLASSIFIER_ARTIFACT`

**Команди**:

```bash
/home/g00n3r/.venvs/base/bin/python - <<'PY'
# inspected TFLite tensors and converted INT8 classifier_head to f32-equivalent weights
PY

cargo build --features microflow32_backend --bin esp32_cl_har
cargo build --bin esp32_cl_har
cargo build --bin online_layer_smoke
. $HOME/export-esp.sh && cargo run --features microflow32_backend --bin esp32_cl_har
espflash monitor --chip esp32 --port /dev/ttyUSB0 --non-interactive --elf target/xtensa-esp32-none-elf/debug/esp32_cl_har
```

**Build result**:

- default firmware build: success
- `microflow32_backend` firmware build: success
- archived `online_layer_smoke` build: success

**Hardware result**:

- flash: success
- app / partition size: `126,512 / 4,128,768 bytes`, тобто `3.06%`
- `espflash flash --monitor` повторно мав monitor-side issue `Failed to initialize input reader`, тому логи знято окремим `espflash monitor`
- `MicroFlow-32` inference still stable: approximately `172.8–174.0 ms`
- `OnlineLayer32.forward()` з pretrained head:
  - first observed call: `161 us`
  - steady calls: approximately `97 us`

Example log:

```text
microflow feature ok: attempt=1, inference_us=173957, input_q0=2, feat0=0, feat1=0.07324092, feat2=5.493069, feat3=0.36620462
online32 forward ok: attempt=1, online_us=161, pred=4(Sitting), confidence=0.9944524
microflow feature ok: attempt=2, inference_us=172868, input_q0=2, feat0=0, feat1=0.07324092, feat2=5.493069, feat3=0.36620462
online32 forward ok: attempt=2, online_us=97, pred=4(Sitting), confidence=0.9944524
```

**Висновок**:

- end-to-end inference-only path тепер дає meaningful class/confidence, а не uniform sanity output
- stationary MPU6050 window predict-иться як `Sitting` з confidence приблизно `0.994`, що узгоджується з нерухомим сенсором
- cost pretrained `OnlineLayer32.forward()` малий порівняно з `MicroFlow-32` feature extraction
- це все ще не CL/adaptation: replay, SGD update у main loop, UART labels і persistence не додавались

**Наступний крок**:

- зробити short movement sanity run: нерухомо / легке переміщення / ходьба з платою в руці, без training
- потім переходити до `ReplayBuffer32` як ізольованого модуля або до supervised-label input тільки після рішення по protocol

## Фаза 4b — Ізольований `ReplayBuffer32` з reservoir-per-class

**Що зроблено**: додано окремий Rust-модуль replay storage для майбутнього CL-loop, не інтегруючи його ще в основний sensor/inference path.

**Додано**:

- [`src/replay_buffer.rs`](/home/g00n3r/projects/esp32_cl_har/src/replay_buffer.rs:1)
  - `ReplayBuffer<const D: usize>`
  - `ReplayBuffer32` і `ReplayBuffer64` alias-и
  - fixed `16` слотів на клас
  - `features[6][16][D]`, `seen[6]`, `len[6]`
  - `push_reservoir(...)` для per-class reservoir sampling
  - `push_fifo(...)` для майбутнього порівняння `FIFO` vs `reservoir`
  - `sample_balanced_batch(...)` для формування replay mini-batch без heap allocation
- [`src/bin/replay_buffer_smoke.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/replay_buffer_smoke.rs:1)
  - synthetic `32`-feature samples
  - заповнення replay buffer по всіх `6` класах
  - balanced batch sampling
  - один `OnlineLayer32.backward_batch()` поверх replay batch
  - без сенсора, inference backend, UART labels, persistence або flash writes

**Рішення**: `ReplayBuffer` зроблено const-generic по feature dimension, але розмір per-class storage лишено фіксованим (`16`) відповідно до research plan. Це дозволяє тримати `MicroFlow-32` як основний embedded path (`12 KB` replay RAM) і не втрачати сумісність з `64`-feature reference path (`24 KB` replay RAM).

**Межі кроку**:

- replay storage ще не підключений у `main.rs`
- labels через UART ще не реалізовані
- runtime CL update у sensor loop ще не виконується
- persistence/NVS не додавались

**Перевірка**:

- `cargo build --bin esp32_cl_har` — success
- `cargo build --features microflow32_backend --bin esp32_cl_har` — success
- `cargo build --bin replay_buffer_smoke` — success
- перша спроба `cargo run --bin replay_buffer_smoke` в sandbox не побачила serial port, але host-level перевірка підтвердила `/dev/ttyUSB0`, `CH340` і `ESP32 rev v3.1`
- `cargo run --bin replay_buffer_smoke` з доступом до `/dev/ttyUSB0` — flash + runtime success

**Hardware log**:

- app / partition size: `106,496 / 4,128,768 bytes`, тобто `2.58%`
- `total_seen=144`
- `total_len=96`, тобто `6 x 16` replay slots заповнено
- `feature_dim=32`
- `fill=499 us`
- `sample=54 us`
- `online_update=633 us`
- `batch_len=12`
- target probability після одного small-LR update змінилась:
  - before: `0.039916247`
  - after: `0.039977703`

**Наступний крок**:

- після цього інтегрувати supervised label protocol або зробити мінімальний synthetic CL checkpoint у firmware, не додаючи persistence до стабілізації update path

## Фаза 4 Scope Correction — PLAN aligned with stable replay-buffer state

**Що виправлено**: після невдалого широкого UART/CL-loop експерименту робоча гілка повернута до стабільної точки `91d9dd3`, де завершено `ReplayBuffer32` smoke, але UART labels і CL loop ще не інтегровані в `main.rs`.

**Поточний фактичний стан**:

- `src/bin/main.rs` лишається стабільним inference path:
  - `MPU6050 -> SlidingWindow -> MicroFlow-32 -> OnlineLayer32.forward() -> logs`
  - без UART labels
  - без replay training у main loop
  - без persistence/NVS/flash writes для CL state
- `ReplayBuffer32` реалізований і smoke-tested окремо:
  - RAM-only storage
  - `FIFO` і `reservoir-per-class` policy в модулі
  - balanced mini-batch sample path
- `PLAN.md` виправлено:
  - persistence позначено як `[DEFERRED / Future Work]`
  - buffer-size ablation лишено optional/future
  - UART labels, RAM-only CL loop і integration into main loop повернуті в pending

**Команди**:

```bash
git branch backup/bad-uart-cl-55b2043 55b2043ccee5863178c579eb700a9e3e0adc2a3c
git stash push -m codex-bad-uart-cleanup-before-stable-branch -- DEVLOG.md src/bin/main.rs
git switch -c stable/phase4-replay-smoke 91d9dd3
. $HOME/export-esp.sh && cargo build
. $HOME/export-esp.sh && cargo build --features microflow32_backend --bin esp32_cl_har
. $HOME/export-esp.sh && timeout 35s cargo run --features microflow32_backend --bin esp32_cl_har
timeout 35s espflash monitor --chip esp32 --port /dev/ttyUSB0 --non-interactive --elf target/xtensa-esp32-none-elf/debug/esp32_cl_har
```

**Build / flash**:

- `cargo build` — success
- `cargo build --features microflow32_backend --bin esp32_cl_har` — success
- firmware flashed successfully
- app / partition size: `126,512 / 4,128,768 bytes`, тобто `3.06%`

**Hardware smoke output**:

```text
mpu6050 detected at 0x68, WHO_AM_I=0x70
phase 3 streaming path ready: backend=microflow-fullconv32-feature-extractor
window buffer ready: 80 samples collected, stride=40
microflow feature ok: attempt=1, inference_us=173957
online32 forward ok: attempt=1, online_us=161, pred=4(Sitting), confidence=0.9944524
microflow feature ok: attempt=10, inference_us=172867
online32 forward ok: attempt=10, online_us=97, pred=4(Sitting), confidence=0.9944524
microflow latency stats: attempts=10, min_us=172867, mean_us=172976, max_us=173957
```

**Висновок**:

- стабільний `main.rs` не зависає після першого prediction
- MPU6050, windowing, MicroFlow-32 і OnlineLayer32 forward працюють на залізі
- наступний ризиковий крок має бути тільки isolated `uart_label_smoke.rs`, без змін у `main.rs`

## Фаза 4c — Isolated UART label smoke

**Що зроблено**: додано окремий binary для перевірки supervised label input через USB serial / UART0 без sensor path, MicroFlow, ReplayBuffer, OnlineLayer або CL update.

**Додано**:

- [`src/bin/uart_label_smoke.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/uart_label_smoke.rs:1)
  - приймає single-character labels `0..5`
  - newline/space/tab ігноруються
  - invalid bytes логуються окремо
  - heartbeat `UART_SMOKE tick=... labels=... invalid=...` підтверджує, що loop не блокується
- `esp-hal` feature `unstable`
  - потрібний для public `Uart::read_buffered(...)`
  - direct UART register hacks не використовувались

**Рішення**: `read_buffered(...)` використано замість `read(...)`, бо `read(...)` у `esp-hal` блокується, якщо FIFO порожній. Це саме та помилка, яка зламала попередню широку інтеграцію в `main.rs`.

**Межі кроку**:

- `main.rs` не змінювався
- labels ще не підключені до `ReplayBuffer`
- CL loop у sensor/inference path ще не інтегрований
- persistence/NVS/flash writes не додавались

**Команди**:

```bash
. $HOME/export-esp.sh && cargo build --bin uart_label_smoke
. $HOME/export-esp.sh && cargo build
. $HOME/export-esp.sh && cargo build --features microflow32_backend --bin esp32_cl_har
. $HOME/export-esp.sh && timeout 35s cargo run --bin uart_label_smoke
```

**Build / flash**:

- `cargo build --bin uart_label_smoke` — success
- `cargo build` — success
- `cargo build --features microflow32_backend --bin esp32_cl_har` — success
- `uart_label_smoke` flashed successfully
- app / partition size: `92,752 / 4,128,768 bytes`, тобто `2.25%`

**Hardware smoke output**:

```text
uart label smoke started
send one-character labels over UART0/USB serial: 0..5
UART_SMOKE tick=1 labels=0 invalid=0
UART_SMOKE tick=7 labels=0 invalid=0
LABEL_RX label=0 name=Walking total_labels=1
LABEL_RX label=4 name=Sitting total_labels=2
LABEL_RX label=5 name=Standing total_labels=3
LABEL_INVALID byte=120 total_invalid=1
UART_SMOKE tick=10 labels=3 invalid=1
UART_SMOKE tick=23 labels=3 invalid=1
```

**Висновок**:

- UART label input працює в isolated smoke
- loop не зависає без input
- логування і прийом labels через той самий UART0/USB serial працюють для короткого smoke
- наступний крок: окремий CL smoke, який з'єднає synthetic/latest feature vector + UART label + `ReplayBuffer32.push(...)`, але все ще не в `main.rs`

## Фаза 4d — Isolated UART + ReplayBuffer + OnlineLayer smoke

**Що зроблено**: додано окремий binary для перевірки стикування `UART label -> ReplayBuffer32.push(...) -> sample_balanced_batch(...) -> OnlineLayer32.backward_batch(...)`.

**Додано**:

- [`src/bin/uart_replay_smoke.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/uart_replay_smoke.rs:1)
  - приймає single-character labels `0..5` через UART0/USB serial
  - генерує synthetic `f32[32]` features для label/sample index
  - додає sample в `ReplayBuffer32`
  - активний policy: `Reservoir`
  - запускає `OnlineLayer32.backward_batch()` кожні `K=10` валідних labels
  - логування `LABEL`, `TRAIN`, `UART_REPLAY_SMOKE`

**Межі кроку**:

- `main.rs` не змінювався
- `MPU6050` не використовується
- `MicroFlow` не використовується
- features synthetic, не real/latest inference features
- persistence/NVS/flash writes не додавались

**Команди**:

```bash
. $HOME/export-esp.sh && cargo build --bin uart_replay_smoke
. $HOME/export-esp.sh && cargo build
. $HOME/export-esp.sh && cargo build --features microflow32_backend --bin esp32_cl_har
. $HOME/export-esp.sh && timeout 45s cargo run --bin uart_replay_smoke
```

**Build / flash**:

- `cargo build --bin uart_replay_smoke` — success
- `cargo build` — success
- `cargo build --features microflow32_backend --bin esp32_cl_har` — success
- `uart_replay_smoke` flashed successfully
- app / partition size: `113,264 / 4,128,768 bytes`, тобто `2.74%`

**Hardware smoke output**:

```text
uart replay smoke started
policy=reservoir, labels_per_update=10, batch_size=12, lr=0.001
UART_REPLAY_SMOKE tick=7 labels=0 train_steps=0 buffer_len=0 invalid=0
LABEL label=0 name=Walking added=1 class_len=1 buffer_len=1 push_us=45 total_seen=1
LABEL label=1 name=Jogging added=1 class_len=1 buffer_len=2 push_us=5 total_seen=2
LABEL label=2 name=Upstairs added=1 class_len=1 buffer_len=3 push_us=2 total_seen=3
LABEL label=3 name=Downstairs added=1 class_len=1 buffer_len=4 push_us=2 total_seen=4
LABEL label=4 name=Sitting added=1 class_len=1 buffer_len=5 push_us=2 total_seen=5
LABEL label=5 name=Standing added=1 class_len=1 buffer_len=6 push_us=1 total_seen=6
LABEL label=0 name=Walking added=1 class_len=2 buffer_len=7 push_us=1 total_seen=7
LABEL label=1 name=Jogging added=1 class_len=2 buffer_len=8 push_us=1 total_seen=8
LABEL label=2 name=Upstairs added=1 class_len=2 buffer_len=9 push_us=1 total_seen=9
LABEL label=3 name=Downstairs added=1 class_len=2 buffer_len=10 push_us=2 total_seen=10
TRAIN policy=reservoir step=1 batch_len=12 sample_us=57 update_us=806 total_seen=10 buffer_len=10
LABEL_INVALID byte=54 total_invalid=1
LABEL_INVALID byte=55 total_invalid=2
LABEL_INVALID byte=56 total_invalid=3
LABEL_INVALID byte=57 total_invalid=4
TRAIN policy=reservoir step=2 batch_len=12 sample_us=11 update_us=560 total_seen=20 buffer_len=20
UART_REPLAY_SMOKE tick=32 labels=20 train_steps=2 buffer_len=20 invalid=4
```

**Висновок**:

- `UART -> ReplayBuffer32 -> OnlineLayer32 update` працює на ESP32 в ізольованому режимі
- `read_buffered()` не блокує loop
- replay insert має малий cost (`~1-45 us`, steady `1-2 us`)
- train update cost у цьому smoke: `560-806 us`
- наступний маленький крок: повторити той самий isolated smoke з `FIFO` policy або зробити compile-time перемикання policy для експериментів, усе ще без `main.rs`

## Фаза 4e — Isolated FIFO policy smoke

**Що зроблено**: `uart_replay_smoke.rs` отримав compile-time перемикач `replay_fifo_policy`, щоб тим самим isolated binary перевіряти `Reservoir` і `FIFO` без дублювання firmware path.

**Зміна**:

- `Cargo.toml`
  - додано feature `replay_fifo_policy = []`
- [`src/bin/uart_replay_smoke.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/uart_replay_smoke.rs:1)
  - default policy: `Reservoir`
  - `--features replay_fifo_policy`: `FIFO`

**Межі кроку**:

- `main.rs` не змінювався
- `MPU6050` не використовується
- `MicroFlow` не використовується
- persistence/NVS/flash writes не додавались

**Команди**:

```bash
. $HOME/export-esp.sh && cargo build --bin uart_replay_smoke
. $HOME/export-esp.sh && cargo build --features replay_fifo_policy --bin uart_replay_smoke
. $HOME/export-esp.sh && cargo build
. $HOME/export-esp.sh && cargo build --features microflow32_backend --bin esp32_cl_har
. $HOME/export-esp.sh && timeout 45s cargo run --features replay_fifo_policy --bin uart_replay_smoke
```

**Build / flash**:

- `cargo build --bin uart_replay_smoke` — success
- `cargo build --features replay_fifo_policy --bin uart_replay_smoke` — success
- `cargo build` — success
- `cargo build --features microflow32_backend --bin esp32_cl_har` — success
- FIFO smoke flashed successfully
- app / partition size: `113,264 / 4,128,768 bytes`, тобто `2.74%`

**Hardware smoke output**:

```text
uart replay smoke started
policy=fifo, labels_per_update=10, batch_size=12, lr=0.001
UART_REPLAY_SMOKE tick=6 labels=0 train_steps=0 buffer_len=0 invalid=0
LABEL label=0 name=Walking added=1 class_len=1 buffer_len=1 push_us=53 total_seen=1
LABEL label=1 name=Jogging added=1 class_len=1 buffer_len=2 push_us=1 total_seen=2
LABEL label=2 name=Upstairs added=1 class_len=1 buffer_len=3 push_us=1 total_seen=3
LABEL label=3 name=Downstairs added=1 class_len=1 buffer_len=4 push_us=1 total_seen=4
LABEL label=4 name=Sitting added=1 class_len=1 buffer_len=5 push_us=1 total_seen=5
LABEL label=5 name=Standing added=1 class_len=1 buffer_len=6 push_us=2 total_seen=6
LABEL label=0 name=Walking added=1 class_len=2 buffer_len=7 push_us=1 total_seen=7
LABEL label=1 name=Jogging added=1 class_len=2 buffer_len=8 push_us=2 total_seen=8
LABEL label=2 name=Upstairs added=1 class_len=2 buffer_len=9 push_us=2 total_seen=9
LABEL label=3 name=Downstairs added=1 class_len=2 buffer_len=10 push_us=2 total_seen=10
TRAIN policy=fifo step=1 batch_len=12 sample_us=53 update_us=810 total_seen=10 buffer_len=10
TRAIN policy=fifo step=2 batch_len=12 sample_us=11 update_us=564 total_seen=20 buffer_len=20
UART_REPLAY_SMOKE tick=32 labels=20 train_steps=2 buffer_len=20 invalid=0
```

**Висновок**:

- isolated `UART -> ReplayBuffer32 -> OnlineLayer32 update` працює і для `FIFO`
- `Reservoir` і `FIFO` тепер обидва мають hardware-smoke підтвердження
- це достатній checkpoint перед наступним малим кроком: інтегрувати RAM-only CL loop у `main.rs`, але тільки з compile-time policy і без persistence

## Фаза 4f — Feature-gated RAM-only CL loop in `main.rs`

**Що зроблено**: RAM-only CL loop інтегровано в основний `MPU6050 -> MicroFlow-32 -> OnlineLayer32` path, але тільки за явним feature gate `cl_uart_labels`.

**Зміна**:

- `Cargo.toml`
  - додано feature `cl_uart_labels = []`
- [`src/bin/main.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/main.rs:1)
  - default `microflow32_backend` лишається inference-only
  - `microflow32_backend,cl_uart_labels` вмикає UART labels + RAM-only replay + train every `K=10`
  - default CL policy: `Reservoir`
  - `replay_fifo_policy` перемикає CL policy на `FIFO`
  - використано `Uart::read_buffered(...)`, не blocking `read(...)`

**Runtime flow у цьому кроці**:

```text
MPU6050
-> SlidingWindow 80 samples
-> quantize i8[240]
-> MicroFlow-32 feature extractor
-> f32[32] features
-> OnlineLayer32.forward()
-> if UART label bytes available:
     ReplayBuffer32.push(label, latest features)
     every K=10 labels: sample batch + OnlineLayer32.backward_batch()
```

**Межі кроку**:

- CL loop вмикається тільки explicit feature `cl_uart_labels`
- persistence/NVS/flash writes не додавались
- protocol лишається single-character labels `0..5`
- без JSON/checksum/timestamps

**Команди**:

```bash
. $HOME/export-esp.sh && cargo build --features microflow32_backend --bin esp32_cl_har
. $HOME/export-esp.sh && cargo build --features microflow32_backend,cl_uart_labels --bin esp32_cl_har
. $HOME/export-esp.sh && cargo build --features microflow32_backend,cl_uart_labels,replay_fifo_policy --bin esp32_cl_har
. $HOME/export-esp.sh && timeout 55s cargo run --features microflow32_backend,cl_uart_labels --bin esp32_cl_har
```

**Build / flash**:

- `cargo build --features microflow32_backend --bin esp32_cl_har` — success
- `cargo build --features microflow32_backend,cl_uart_labels --bin esp32_cl_har` — success
- `cargo build --features microflow32_backend,cl_uart_labels,replay_fifo_policy --bin esp32_cl_har` — success
- feature-gated CL main flashed successfully
- app / partition size: `135,424 / 4,128,768 bytes`, тобто `3.28%`

**Hardware smoke output**:

```text
mpu6050 detected at 0x68, WHO_AM_I=0x70
phase 3 streaming path ready: backend=microflow-fullconv32-feature-extractor
phase 4 RAM-only CL enabled: labels=UART0/GPIO3, policy=reservoir, labels_per_update=10, batch_size=12, lr=0.001, persistence=off
microflow feature ok: attempt=1, inference_us=173121
online32 forward ok: attempt=1, online_us=184, pred=4(Sitting), confidence=0.9944524
microflow feature ok: attempt=4, inference_us=172479
online32 forward ok: attempt=4, online_us=72, pred=4(Sitting), confidence=0.9945515
LABEL label=4 name=Sitting added=1 class_len=1 buffer_len=1 push_us=45 total_seen=1 attempt=4
LABEL label=4 name=Sitting added=1 class_len=8 buffer_len=8 push_us=6 total_seen=8 attempt=4
microflow feature ok: attempt=5, inference_us=172507
online32 forward ok: attempt=5, online_us=89, pred=4(Sitting), confidence=0.9944524
LABEL label=4 name=Sitting added=1 class_len=10 buffer_len=10 push_us=6 total_seen=10 attempt=5
TRAIN policy=reservoir step=1 batch_len=12 sample_us=59 update_us=665 total_seen=10 buffer_len=10 attempt=5
microflow latency stats: attempts=10, min_us=172479, mean_us=172558, max_us=173121
microflow feature ok: attempt=17, inference_us=172499
online32 forward ok: attempt=17, online_us=89, pred=4(Sitting), confidence=0.9944629
```

**Висновок**:

- feature-gated RAM-only CL loop працює в `main.rs`
- `UART label -> latest MicroFlow-32 features -> ReplayBuffer32 -> OnlineLayer32.backward_batch()` підтверджено на ESP32
- loop не завис після labels/train і продовжив inference до timeout
- CL overhead лишається малим порівняно з MicroFlow-32 feature extraction:
  - MicroFlow-32: приблизно `172.5 ms`
  - OnlineLayer forward: приблизно `72-184 us`
  - replay push: steady `~2-6 us`
  - train update: `665 us` у цьому smoke
- наступний маленький крок: hardware smoke `main.rs` з `cl_uart_labels,replay_fifo_policy`, щоб підтвердити FIFO у повному sensor/inference loop

## Фаза 4g — Full `main.rs` CL smoke with FIFO policy

**Що зроблено**: перевірено повний `main.rs` RAM-only CL loop у `FIFO` режимі через feature flags `microflow32_backend,cl_uart_labels,replay_fifo_policy`.

**Межі кроку**:

- нової логіки не додавалось
- `main.rs` уже мав feature-gated CL loop з попереднього кроку
- перевірка сфокусована тільки на FIFO policy у повному sensor/inference loop
- persistence/NVS/flash writes не додавались

**Команда**:

```bash
. $HOME/export-esp.sh && timeout 55s cargo run --features microflow32_backend,cl_uart_labels,replay_fifo_policy --bin esp32_cl_har
```

**Build / flash**:

- build перед flash — success
- FIFO CL main flashed successfully
- app / partition size: `135,424 / 4,128,768 bytes`, тобто `3.28%`

**Hardware smoke output**:

```text
mpu6050 detected at 0x68, WHO_AM_I=0x70
phase 3 streaming path ready: backend=microflow-fullconv32-feature-extractor
phase 4 RAM-only CL enabled: labels=UART0/GPIO3, policy=fifo, labels_per_update=10, batch_size=12, lr=0.001, persistence=off
microflow feature ok: attempt=1, inference_us=173143
online32 forward ok: attempt=1, online_us=176, pred=4(Sitting), confidence=0.994474
LABEL label=4 name=Sitting added=1 class_len=1 buffer_len=1 push_us=50 total_seen=1 attempt=3
LABEL label=4 name=Sitting added=1 class_len=8 buffer_len=8 push_us=6 total_seen=8 attempt=3
LABEL label=4 name=Sitting added=1 class_len=10 buffer_len=10 push_us=10 total_seen=10 attempt=4
TRAIN policy=fifo step=1 batch_len=12 sample_us=63 update_us=685 total_seen=10 buffer_len=10 attempt=4
microflow latency stats: attempts=10, min_us=172469, mean_us=172563, max_us=173143
microflow feature ok: attempt=17, inference_us=172496
online32 forward ok: attempt=17, online_us=105, pred=4(Sitting), confidence=0.994463
```

**Висновок**:

- full `main.rs` CL loop працює і з `FIFO`
- після `TRAIN policy=fifo` firmware не завис і продовжив inference до timeout
- `reservoir` і `FIFO` тепер обидва підтверджені у повному sensor/inference loop
- наступний маленький крок: додати мінімальний experiment-mode logging для `No adaptation / FIFO / Reservoir`, щоб короткі сесії було легше порівнювати без зміни математики

## Фаза 4h — Stable experiment log tags

**Що зроблено**: додано стабільні plain-text log tags для парсингу коротких експериментальних сесій `no_adapt / reservoir / fifo`.

**Зміна**:

- [`src/bin/main.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/main.rs:1)
  - startup logs:
    - `EXPERIMENT ...`
    - `RESOURCE ...`
  - prediction logs:
    - `PRED mode=... attempt=... class=... label=... conf=... infer_us=... head_us=...`
  - CL logs:
    - `LABEL mode=... label=... buffer_len=... push_us=...`
    - `TRAIN mode=... policy=... batch_len=... sample_us=... update_us=...`

**Межі кроку**:

- математика не змінювалась
- replay policy не змінювався
- UART protocol не змінювався
- persistence/NVS/flash writes не додавались

**Команди**:

```bash
. $HOME/export-esp.sh && cargo build --features microflow32_backend --bin esp32_cl_har
. $HOME/export-esp.sh && cargo build --features microflow32_backend,cl_uart_labels --bin esp32_cl_har
. $HOME/export-esp.sh && cargo build --features microflow32_backend,cl_uart_labels,replay_fifo_policy --bin esp32_cl_har
. $HOME/export-esp.sh && timeout 35s cargo run --features microflow32_backend --bin esp32_cl_har
. $HOME/export-esp.sh && timeout 45s cargo run --features microflow32_backend,cl_uart_labels --bin esp32_cl_har
```

**Build / flash**:

- `cargo build --features microflow32_backend --bin esp32_cl_har` — success
- `cargo build --features microflow32_backend,cl_uart_labels --bin esp32_cl_har` — success
- `cargo build --features microflow32_backend,cl_uart_labels,replay_fifo_policy --bin esp32_cl_har` — success
- no-adapt main flashed successfully
- reservoir CL main flashed successfully
- no-adapt app / partition size: `127,408 / 4,128,768 bytes`, тобто `3.09%`
- reservoir CL app / partition size: `136,384 / 4,128,768 bytes`, тобто `3.30%`

**No-adapt hardware output**:

```text
EXPERIMENT mode=no_adapt labels=off policy=none feature_dim=32 persistence=off
RESOURCE mode=no_adapt replay_ram_est=0 feature_dim=32 slots_per_class=0 batch_size=0 persistence=off
PRED mode=no_adapt attempt=1 class=4 label=Sitting conf=0.994474 infer_us=173103 head_us=160
PRED mode=no_adapt attempt=8 class=4 label=Sitting conf=0.9944524 infer_us=172396 head_us=105
```

**Reservoir CL hardware output**:

```text
EXPERIMENT mode=reservoir labels=uart policy=reservoir feature_dim=32 labels_per_update=10 persistence=off
RESOURCE mode=reservoir replay_ram_est=12288 feature_dim=32 slots_per_class=16 batch_size=12 persistence=off
PRED mode=reservoir attempt=1 class=4 label=Sitting conf=0.9944524 infer_us=173131 head_us=180
LABEL mode=reservoir label=4 name=Sitting added=1 class_len=1 buffer_len=1 push_us=61 total_seen=1 attempt=5
LABEL mode=reservoir label=4 name=Sitting added=1 class_len=10 buffer_len=10 push_us=5 total_seen=10 attempt=6
TRAIN mode=reservoir policy=reservoir step=1 batch_len=12 sample_us=59 update_us=685 total_seen=10 buffer_len=10 attempt=6
PRED mode=reservoir attempt=13 class=4 label=Sitting conf=0.99455464 infer_us=172405 head_us=93
```

**Висновок**:

- `no_adapt` і `reservoir` мають grep-friendly logs для experiment parsing
- `RESOURCE` вже містить estimated replay RAM (`12288` bytes для `6 x 16 x 32 x f32`)
- `PRED/LABEL/TRAIN` можна напряму парсити Python-скриптом у таблиці latency/update/resource
- наступний маленький крок: зробити короткий parser script або notebook cell для цих log tags перед довшими сесіями

## Фаза 5a — Experiment log parser

**Що зроблено**: додано stdlib-only parser для stable firmware log tags, щоб raw serial logs перетворювати в CSV таблиці для експериментів і статті.

**Додано**:

- [`scripts/parse_experiment_logs.py`](/home/g00n3r/projects/esp32_cl_har/scripts/parse_experiment_logs.py:1)
  - парсить `EXPERIMENT`, `RESOURCE`, `PRED`, `LABEL`, `TRAIN`
  - прибирає ANSI color codes з `espflash monitor`
  - ігнорує bootloader/debug text і markdown placeholders
  - пише окремі CSV:
    - `<stem>_experiment.csv`
    - `<stem>_resource.csv`
    - `<stem>_pred.csv`
    - `<stem>_label.csv`
    - `<stem>_train.csv`
  - пише `<stem>_summary.json` з counts і latency/update summary

**Приклад використання**:

```bash
python3 scripts/parse_experiment_logs.py logs/no_adapt_session.txt --out-dir parsed_logs/no_adapt
python3 scripts/parse_experiment_logs.py logs/reservoir_session.txt --out-dir parsed_logs/reservoir
python3 scripts/parse_experiment_logs.py logs/fifo_session.txt --out-dir parsed_logs/fifo
```

**Межі кроку**:

- firmware не змінювалась
- `main.rs` не змінювався
- parser не потребує `pandas`
- plots ще не генеруються

**Команди**:

```bash
python3 -m py_compile scripts/parse_experiment_logs.py
python3 scripts/parse_experiment_logs.py DEVLOG.md --out-dir /tmp/esp32_cl_har_parsed_devlog
```

**Smoke output**:

```text
rows:
  EXPERIMENT: 2
  RESOURCE: 2
  PRED: 4
  LABEL: 28
  TRAIN: 8
modes:
  no_adapt
  reservoir
pred_infer_us mean: 172758.75
pred_head_us mean: 134.5
train_update_us mean: 682.14
```

**Generated files у smoke test**:

```text
DEVLOG_experiment.csv
DEVLOG_label.csv
DEVLOG_pred.csv
DEVLOG_resource.csv
DEVLOG_summary.json
DEVLOG_train.csv
```

**Висновок**:

- parser готовий для коротких і довших experiment sessions
- наступний маленький крок: зробити перші контрольовані raw log captures для `no_adapt`, `reservoir`, `fifo`, а потім прогнати їх через parser

## Фаза 5b — Controlled dry-run captures for no_adapt / reservoir / FIFO

**Що зроблено**: виконано короткий контрольований hardware dry-run для трьох режимів і пропарсено raw serial logs у CSV/JSON.

**Межі кроку**:

- firmware не змінювалась
- `main.rs` не змінювався
- git не чіпався
- сенсор лежав нерухомо; це не фінальний accuracy experiment, а перевірка pipeline збору логів
- для CL dry-run надсилались labels `4` (`Sitting`), бо плата й MPU6050 були нерухомі

**Команди**:

```bash
script -q -c 'timeout 35s espflash monitor --chip esp32 --port /dev/ttyUSB0 --non-interactive --elf target/xtensa-esp32-none-elf/debug/esp32_cl_har' logs/raw/no_adapt_dryrun_2026-05-09.txt

script -q -c "timeout 55s bash -lc '. $HOME/export-esp.sh && cargo run --features microflow32_backend,cl_uart_labels --bin esp32_cl_har'" logs/raw/reservoir_dryrun_2026-05-09.txt

script -q -c "timeout 55s bash -lc '. $HOME/export-esp.sh && cargo run --features microflow32_backend,cl_uart_labels,replay_fifo_policy --bin esp32_cl_har'" logs/raw/fifo_dryrun_2026-05-09.txt

python3 scripts/parse_experiment_logs.py logs/raw/no_adapt_dryrun_2026-05-09.txt --out-dir logs/parsed/no_adapt
python3 scripts/parse_experiment_logs.py logs/raw/reservoir_dryrun_2026-05-09.txt --out-dir logs/parsed/reservoir
python3 scripts/parse_experiment_logs.py logs/raw/fifo_dryrun_2026-05-09.txt --out-dir logs/parsed/fifo
```

**Hardware output summary**:

```text
no_adapt:
  rows: EXPERIMENT=1 RESOURCE=1 PRED=11 LABEL=0 TRAIN=0
  pred_infer_us mean=172460.45 min=172396 max=173103
  pred_head_us mean=110.09 min=104 max=160
  resource replay_ram_est=0

reservoir:
  app_size=136384 / 4128768 bytes = 3.30%
  rows: EXPERIMENT=1 RESOURCE=1 PRED=17 LABEL=10 TRAIN=1
  pred_infer_us mean=172413.06 min=172294 max=173131
  pred_head_us mean=94.35 min=84 max=180
  label_push_us mean=16.2 min=6 max=62
  train_sample_us=59
  train_update_us=681
  resource replay_ram_est=12288

fifo:
  app_size=136384 / 4128768 bytes = 3.30%
  rows: EXPERIMENT=1 RESOURCE=1 PRED=17 LABEL=10 TRAIN=1
  pred_infer_us mean=172414.35 min=172306 max=173147
  pred_head_us mean=103.29 min=96 max=176
  label_push_us mean=14.0 min=6 max=53
  train_sample_us=59
  train_update_us=669
  resource replay_ram_est=12288
```

**Generated files**:

```text
logs/raw/no_adapt_dryrun_2026-05-09.txt
logs/raw/reservoir_dryrun_2026-05-09.txt
logs/raw/fifo_dryrun_2026-05-09.txt

logs/parsed/no_adapt/no_adapt_dryrun_2026-05-09_*.csv/json
logs/parsed/reservoir/reservoir_dryrun_2026-05-09_*.csv/json
logs/parsed/fifo/fifo_dryrun_2026-05-09_*.csv/json
```

**Висновок**:

- collection pipeline працює end-to-end: raw serial log -> parser -> CSV/summary JSON
- `no_adapt`, `reservoir`, `fifo` мають стабільні `EXPERIMENT / RESOURCE / PRED / LABEL / TRAIN` rows
- CL overhead у dry-run дуже малий відносно MicroFlow-32 inference: update приблизно `0.67-0.68 ms` проти feature extraction приблизно `172 ms`
- наступний маленький крок: провести короткий pilot experiment з реальною розміткою рухів або записати rehearsal protocol для фінальних сесій

## Фаза 5c — Dry-run comparison table and 2-class pilot protocol

**Що зроблено**: додано helper для зведення кількох parsed summary JSON у одну comparison CSV і зафіксовано перший реальний pilot protocol.

**Додано**:

- [`scripts/summarize_experiment_runs.py`](/home/g00n3r/projects/esp32_cl_har/scripts/summarize_experiment_runs.py:1)
  - читає `*_summary.json` після `parse_experiment_logs.py`
  - формує одну CSV-таблицю для `no_adapt / fifo / reservoir`
  - виводить `pred_rows`, `label_rows`, `train_rows`, `replay_ram_est`, latency means, update means
  - рахує `cl_update_vs_infer_pct`
  - сортує режими в порядку `no_adapt -> fifo -> reservoir`

**Команди**:

```bash
python3 -m py_compile scripts/summarize_experiment_runs.py

python3 scripts/summarize_experiment_runs.py \
  logs/parsed/no_adapt/no_adapt_dryrun_2026-05-09_summary.json \
  logs/parsed/reservoir/reservoir_dryrun_2026-05-09_summary.json \
  logs/parsed/fifo/fifo_dryrun_2026-05-09_summary.json \
  --out-csv logs/parsed/dryrun_comparison_2026-05-09.csv
```

**Generated comparison CSV**:

```text
logs/parsed/dryrun_comparison_2026-05-09.csv
```

**Dry-run comparison summary**:

```text
mode       pred_rows  label_rows  train_rows  replay_ram_est  infer_us_mean  train_update_us  update_vs_infer
no_adapt   11         0           0           0               172460.455     -                -
fifo       17         10          1           12288           172414.353     669              0.388%
reservoir  17         10          1           12288           172413.059     681              0.395%
```

**Pilot 2-class protocol**:

Мета: не фінальна accuracy, а перший реальний sanity check для adaptation на русі.

```text
Classes:
  0 = Walking label slot, used here for standing-like small movement
  4 = Sitting

Fixed firmware settings:
  feature_dim = 32
  labels_per_update K = 10
  batch_size = 12
  replay slots = 16/class
  persistence = off

Modes:
  1. no_adapt
  2. fifo
  3. reservoir

Recommended capture:
  no_adapt:
    - 30-60 s Sitting, no labels
    - 30-60 s standing-like small movement, no labels

  fifo:
    - start stationary Sitting, send 10-20 labels "4"
    - standing-like small movement, send 10-20 labels "0"
    - keep logging predictions after at least one TRAIN

  reservoir:
    - same sequence as fifo
    - same approximate movement and label counts

Label input:
  send single characters over UART:
    4 for Sitting
    0 for the second standing-like movement segment
```

**Висновок**:

- dry-run summaries тепер зводяться в одну таблицю без ручного копіювання
- наступний execution step: провести короткий `Sitting` vs standing-like small movement pilot capture і пропарсити його тим самим pipeline

## Фаза 5d — 2-class pilot capture: Sitting vs standing-like small movement

**Що зроблено**: виконано перший реальний 2-class pilot з фізичним рухом сенсора для `no_adapt`, `fifo`, `reservoir`.

**Межі кроку**:

- firmware не змінювалась
- `main.rs` не змінювався
- persistence/NVS/flash state не додавались
- це pilot sanity check, не фінальний accuracy experiment
- другий segment фізично був standing-like small movement, а не повноцінний `Walking`: рух був обмежений USB-кабелем біля комп'ютера
- labels для CL надсилались burst-ами:
  - `4444444444` для `Sitting`
  - `0000000000` для другого standing-like movement segment
- через burst labels частина labels прив'язана до latest feature vector, а не рівномірно до всього segment; для фінальних сесій потрібен рівномірніший label protocol

**Команди**:

```bash
script -q -c "timeout 110s bash -lc '. $HOME/export-esp.sh && cargo run --features microflow32_backend --bin esp32_cl_har'" logs/raw/pilot_2class/no_adapt_pilot_2class_2026-05-09.txt

python3 scripts/parse_experiment_logs.py logs/raw/pilot_2class/no_adapt_pilot_2class_2026-05-09.txt --out-dir logs/parsed/pilot_2class/no_adapt

script -q -c "timeout 120s bash -lc '. $HOME/export-esp.sh && cargo run --features microflow32_backend,cl_uart_labels,replay_fifo_policy --bin esp32_cl_har'" logs/raw/pilot_2class/fifo_pilot_2class_2026-05-09.txt

python3 scripts/parse_experiment_logs.py logs/raw/pilot_2class/fifo_pilot_2class_2026-05-09.txt --out-dir logs/parsed/pilot_2class/fifo

script -q -c "timeout 120s bash -lc '. $HOME/export-esp.sh && cargo run --features microflow32_backend,cl_uart_labels --bin esp32_cl_har'" logs/raw/pilot_2class/reservoir_pilot_2class_2026-05-09.txt

python3 scripts/parse_experiment_logs.py logs/raw/pilot_2class/reservoir_pilot_2class_2026-05-09.txt --out-dir logs/parsed/pilot_2class/reservoir

python3 scripts/summarize_experiment_runs.py \
  logs/parsed/pilot_2class/no_adapt/no_adapt_pilot_2class_2026-05-09_summary.json \
  logs/parsed/pilot_2class/fifo/fifo_pilot_2class_2026-05-09_summary.json \
  logs/parsed/pilot_2class/reservoir/reservoir_pilot_2class_2026-05-09_summary.json \
  --out-csv logs/parsed/pilot_2class/pilot_2class_comparison_2026-05-09.csv
```

**Generated files**:

```text
logs/raw/pilot_2class/no_adapt_pilot_2class_2026-05-09.txt
logs/raw/pilot_2class/fifo_pilot_2class_2026-05-09.txt
logs/raw/pilot_2class/reservoir_pilot_2class_2026-05-09.txt

logs/parsed/pilot_2class/no_adapt/no_adapt_pilot_2class_2026-05-09_*.csv/json
logs/parsed/pilot_2class/fifo/fifo_pilot_2class_2026-05-09_*.csv/json
logs/parsed/pilot_2class/reservoir/reservoir_pilot_2class_2026-05-09_*.csv/json
logs/parsed/pilot_2class/pilot_2class_comparison_2026-05-09.csv
```

**Pilot comparison summary**:

```text
mode       pred_rows  label_rows  train_rows  replay_ram_est  infer_us_mean  train_update_us_mean  update_vs_infer
no_adapt   45         0           0           0               172508.289     -                     -
fifo       50         20          2           12288           172416.680     664.5                 0.385%
reservoir  50         20          2           12288           172274.080     658.0                 0.382%
```

**Prediction class distribution**:

```text
no_adapt:
  Sitting=40
  Standing=5

fifo:
  Sitting=43
  Standing=7

reservoir:
  Sitting=21
  Standing=17
  Upstairs=12
```

**Hardware observations**:

- усі три режими booted, MPU6050 detected, MicroFlow-32 inference працював
- FIFO і reservoir прийняли по `20` labels і зробили по `2` train updates
- firmware не зависала після train updates
- standing-like movement чітко видно в accelerometer values і feature changes
- другий segment був standing-like small movement rather than full `Walking`, тому predictions як `Standing`/`Upstairs` є правдоподібними і не мають інтерпретуватись як Walking failure
- reservoir після labels другого segment сильніше змістив prediction distribution, ніж FIFO

**Висновок**:

- experimental pipeline для реального руху працює end-to-end
- CL overhead лишається дуже малим: приблизно `0.38-0.39%` від MicroFlow-32 inference time
- поточний pilot підтвердив real-device feasibility для supervised RAM-only CL loop, але не є фінальним HAR accuracy result
- full `Walking` accuracy можна залишити на later/optional experiment з кращою фізичною свободою руху
- наступний маленький крок: додати post-processing script, який оцінює predictions по вручну заданих часових/attempt сегментах

## Фаза 5e — Segment-level evaluator for pilot runs

**Що зроблено**: додано post-processing script для оцінки parsed `PRED` rows по вручну заданих attempt ranges.

**Додано**:

- [`scripts/evaluate_segments.py`](/home/g00n3r/projects/esp32_cl_har/scripts/evaluate_segments.py:1)
  - читає parsed `*_pred.csv`
  - приймає segment specs у форматі `name:start:end:accepted_label|accepted_label`
  - рахує `rows`, `accepted_rows`, `accepted_rate`, `mean_conf`, `pred_counts`
  - не потребує `pandas`
  - не змінює firmware або raw logs

**Команди**:

```bash
python3 -m py_compile scripts/evaluate_segments.py

python3 scripts/evaluate_segments.py \
  logs/parsed/pilot_2class/no_adapt/no_adapt_pilot_2class_2026-05-09_pred.csv \
  --segment sitting:1:6:Sitting \
  --segment standing_like:7:45:Standing\|Upstairs \
  --out-csv logs/parsed/pilot_2class/no_adapt/no_adapt_pilot_2class_2026-05-09_segments.csv

python3 scripts/evaluate_segments.py \
  logs/parsed/pilot_2class/fifo/fifo_pilot_2class_2026-05-09_pred.csv \
  --segment sitting:1:15:Sitting \
  --segment standing_like:16:50:Standing\|Upstairs \
  --out-csv logs/parsed/pilot_2class/fifo/fifo_pilot_2class_2026-05-09_segments.csv

python3 scripts/evaluate_segments.py \
  logs/parsed/pilot_2class/reservoir/reservoir_pilot_2class_2026-05-09_pred.csv \
  --segment sitting:1:16:Sitting \
  --segment standing_like:17:50:Standing\|Upstairs \
  --out-csv logs/parsed/pilot_2class/reservoir/reservoir_pilot_2class_2026-05-09_segments.csv
```

**Generated files**:

```text
logs/parsed/pilot_2class/no_adapt/no_adapt_pilot_2class_2026-05-09_segments.csv
logs/parsed/pilot_2class/fifo/fifo_pilot_2class_2026-05-09_segments.csv
logs/parsed/pilot_2class/reservoir/reservoir_pilot_2class_2026-05-09_segments.csv
logs/parsed/pilot_2class/pilot_2class_segment_eval_2026-05-09.csv
```

**Segment evaluation summary**:

```text
mode       segment        attempts  accepted labels     accepted_rate  pred_counts
no_adapt   sitting        1-6       Sitting             1.0000         Sitting=6
no_adapt   standing_like  7-45      Standing|Upstairs   0.1282         Sitting=34;Standing=5
fifo       sitting        1-15      Sitting             1.0000         Sitting=15
fifo       standing_like  16-50     Standing|Upstairs   0.2000         Sitting=28;Standing=7
reservoir  sitting        1-16      Sitting             1.0000         Sitting=16
reservoir  standing_like  17-50     Standing|Upstairs   0.8529         Sitting=5;Standing=17;Upstairs=12
```

**Висновок**:

- segment-level evaluator підтвердив, що stationary `Sitting` стабільно відпрацьовує у всіх трьох режимах
- standing-like segment не є full HAR accuracy benchmark, але показує помітну зміну prediction distribution після reservoir adaptation
- це корисний pilot sanity result для real-device feasibility section
- наступний маленький крок: згенерувати просту таблицю/plot для resource + segment pilot results або перейти до тексту `Experimental Setup / Results`

## Фаза 5f — Paper-ready pilot result tables

**Що зроблено**: додано generator для компактних Markdown-таблиць з resource/CL overhead і segment-level pilot summary.

**Додано**:

- [`scripts/build_pilot_results_tables.py`](/home/g00n3r/projects/esp32_cl_har/scripts/build_pilot_results_tables.py:1)
  - читає `pilot_2class_comparison_*.csv`
  - читає `pilot_2class_segment_eval_*.csv`
  - формує paper-ready Markdown tables
  - форматує inference у `ms`, OnlineLayer/update у `us`, replay RAM у `KiB`
  - екранує Markdown pipe characters у labels
  - не потребує `pandas`

**Команди**:

```bash
python3 -m py_compile scripts/build_pilot_results_tables.py

python3 scripts/build_pilot_results_tables.py \
  --comparison-csv logs/parsed/pilot_2class/pilot_2class_comparison_2026-05-09.csv \
  --segment-csv logs/parsed/pilot_2class/pilot_2class_segment_eval_2026-05-09.csv \
  --out-md results/tables/phase5_pilot_results_2026-05-09.md
```

**Generated file**:

```text
results/tables/phase5_pilot_results_2026-05-09.md
```

**Table snapshot**:

```text
Resource And CL Overhead:
no_adapt   infer=172.51 ms  replay=0        train=-
fifo       infer=172.42 ms  replay=12 KiB   train=664.5 us  update/infer=0.385%
reservoir  infer=172.27 ms  replay=12 KiB   train=658.0 us  update/infer=0.382%

Segment-Level Pilot Summary:
no_adapt   standing_like accepted_rate=12.8%
fifo       standing_like accepted_rate=20.0%
reservoir  standing_like accepted_rate=85.3%
```

**Висновок**:

- resource/pilot results тепер готові для перенесення в `Results`
- головний publishable signal: RAM-only CL update overhead менший за `1%` від MicroFlow-32 inference time
- pilot segment result треба описувати як feasibility/sanity check, не як фінальну 6-class HAR accuracy
- наступний маленький крок: почати draft секцій `Experimental Setup` і `Results` або зробити мінімальні plots з цих CSV

## Фаза 5g — Sitting vs upstairs-like vertical hand-motion pilot

**Що зроблено**: виконано контрольований 2-class real-device pilot для `Sitting` vs upstairs-like vertical hand-motion.

**Межі кроку**:

- firmware не змінювалась
- `main.rs` не змінювався
- parser/scripts не змінювались
- persistence/NVS/flash state не додавались
- це не real staircase benchmark і не фінальна 6-class HAR accuracy
- другий segment є upward hand-motion біля ПК, обмежений USB-кабелем

**Protocol**:

```text
Segment 1:
  Sitting / stationary
  label = 4

Segment 2:
  upstairs-like vertical hand-motion
  label = 2

Modes:
  no_adapt: no labels, PRED only
  fifo: 4444444444 -> TRAIN step=1, then 2222222222 -> TRAIN step=2
  reservoir: same as fifo
```

**Команди**:

```bash
mkdir -p logs/raw/pilot_sit_up \
  logs/parsed/pilot_sit_up/no_adapt \
  logs/parsed/pilot_sit_up/fifo \
  logs/parsed/pilot_sit_up/reservoir

script -q -c "timeout 110s bash -lc '. $HOME/export-esp.sh && cargo run --features microflow32_backend --bin esp32_cl_har'" logs/raw/pilot_sit_up/sit_up_no_adapt_2026-05-09.txt

script -q -c "timeout 120s bash -lc '. $HOME/export-esp.sh && cargo run --features microflow32_backend,cl_uart_labels,replay_fifo_policy --bin esp32_cl_har'" logs/raw/pilot_sit_up/sit_up_fifo_2026-05-09.txt

script -q -c "timeout 120s bash -lc '. $HOME/export-esp.sh && cargo run --features microflow32_backend,cl_uart_labels --bin esp32_cl_har'" logs/raw/pilot_sit_up/sit_up_reservoir_2026-05-09.txt

python3 scripts/parse_experiment_logs.py logs/raw/pilot_sit_up/sit_up_no_adapt_2026-05-09.txt --out-dir logs/parsed/pilot_sit_up/no_adapt
python3 scripts/parse_experiment_logs.py logs/raw/pilot_sit_up/sit_up_fifo_2026-05-09.txt --out-dir logs/parsed/pilot_sit_up/fifo
python3 scripts/parse_experiment_logs.py logs/raw/pilot_sit_up/sit_up_reservoir_2026-05-09.txt --out-dir logs/parsed/pilot_sit_up/reservoir

python3 scripts/summarize_experiment_runs.py \
  logs/parsed/pilot_sit_up/no_adapt/sit_up_no_adapt_2026-05-09_summary.json \
  logs/parsed/pilot_sit_up/fifo/sit_up_fifo_2026-05-09_summary.json \
  logs/parsed/pilot_sit_up/reservoir/sit_up_reservoir_2026-05-09_summary.json \
  --out-csv logs/parsed/pilot_sit_up/sit_up_comparison_2026-05-09.csv

python3 scripts/evaluate_segments.py \
  logs/parsed/pilot_sit_up/no_adapt/sit_up_no_adapt_2026-05-09_pred.csv \
  --segment sitting:1:18:Sitting \
  --segment upstairs_like:19:45:Upstairs\|Downstairs \
  --out-csv logs/parsed/pilot_sit_up/no_adapt/sit_up_no_adapt_2026-05-09_segments.csv

python3 scripts/evaluate_segments.py \
  logs/parsed/pilot_sit_up/fifo/sit_up_fifo_2026-05-09_pred.csv \
  --segment sitting:1:15:Sitting \
  --segment upstairs_like:16:50:Upstairs\|Downstairs \
  --out-csv logs/parsed/pilot_sit_up/fifo/sit_up_fifo_2026-05-09_segments.csv

python3 scripts/evaluate_segments.py \
  logs/parsed/pilot_sit_up/reservoir/sit_up_reservoir_2026-05-09_pred.csv \
  --segment sitting:1:17:Sitting \
  --segment upstairs_like:18:50:Upstairs\|Downstairs \
  --out-csv logs/parsed/pilot_sit_up/reservoir/sit_up_reservoir_2026-05-09_segments.csv
```

**Generated files**:

```text
logs/raw/pilot_sit_up/sit_up_no_adapt_2026-05-09.txt
logs/raw/pilot_sit_up/sit_up_fifo_2026-05-09.txt
logs/raw/pilot_sit_up/sit_up_reservoir_2026-05-09.txt

logs/parsed/pilot_sit_up/no_adapt/sit_up_no_adapt_2026-05-09_*.csv/json
logs/parsed/pilot_sit_up/fifo/sit_up_fifo_2026-05-09_*.csv/json
logs/parsed/pilot_sit_up/reservoir/sit_up_reservoir_2026-05-09_*.csv/json
logs/parsed/pilot_sit_up/sit_up_comparison_2026-05-09.csv
logs/parsed/pilot_sit_up/sit_up_segment_eval_2026-05-09.csv
```

**Resource / overhead summary**:

```text
mode       pred_rows  labels  train_updates  replay_ram  infer_us_mean  train_update_us_mean  update_vs_infer
no_adapt   45         0       0              0           172511.911     -                     -
fifo       50         20      2              12288       172289.640     666.5                 0.387%
reservoir  50         20      2              12288       172324.380     656.0                 0.381%
```

**Segment evaluation summary**:

```text
mode       segment        attempts  accepted labels       accepted_rate  pred_counts
no_adapt   sitting        1-18      Sitting               1.0000         Sitting=18
no_adapt   upstairs_like  19-45     Upstairs|Downstairs   0.0000         Sitting=27
fifo       sitting        1-15      Sitting               1.0000         Sitting=15
fifo       upstairs_like  16-50     Upstairs|Downstairs   0.8857         Sitting=4;Upstairs=31
reservoir  sitting        1-17      Sitting               1.0000         Sitting=17
reservoir  upstairs_like  18-50     Upstairs|Downstairs   0.9394         Downstairs=4;Sitting=2;Upstairs=27
```

**Hardware observations**:

- усі три режими booted, MPU6050 detected, MicroFlow-32 inference працював
- FIFO і reservoir прийняли labels `4` і `2`
- FIFO і reservoir зробили по `2` train updates
- firmware не зависала після train updates
- no_adapt залишив upstairs-like segment як `Sitting`, але confidence помітно падала
- FIFO і reservoir на upstairs-like vertical motion стабільно зміщували predictions у `Upstairs`/`Downstairs`
- CL update overhead лишився приблизно `0.38%` від MicroFlow-32 inference time

**Висновок**:

- цей pilot дає сильніший real-device motion validation, ніж попередній standing-like pilot
- результат не треба описувати як staircase benchmark
- коректний claim: ESP32 RAM-only CL loop реагує на supervised labels `4/2` і змінює prediction distribution на upstairs-like vertical hand-motion без runtime failure
- next step: згенерувати paper-ready таблиці для `pilot_sit_up` або починати `Experimental Setup / Results` draft з двома pilot blocks

## Фаза 5h — Paper results analysis notebook

**Що зроблено**: створено Jupyter notebook для paper-ready analysis і plots на основі вже parsed CSV.

**Додано**:

- [`notebooks/paper_results_analysis.ipynb`](/home/g00n3r/projects/esp32_cl_har/notebooks/paper_results_analysis.ipynb:1)

**Scope**:

- firmware не змінювалась
- `main.rs` не змінювався
- raw logs не змінювались
- hardware experiments не перезапускались
- notebook працює тільки з existing parsed logs / CSV outputs

**Expected inputs**:

```text
logs/parsed/pilot_sit_up/sit_up_comparison_2026-05-09.csv
logs/parsed/pilot_sit_up/sit_up_segment_eval_2026-05-09.csv
logs/parsed/pilot_2class/pilot_2class_comparison_2026-05-09.csv
logs/parsed/pilot_2class/pilot_2class_segment_eval_2026-05-09.csv
results/tables/phase5_pilot_results_2026-05-09.md
```

**Notebook outputs when run**:

```text
results/tables/table_resource_overhead_sit_up.csv
results/tables/table_resource_overhead_sit_up.md
results/tables/table_prediction_distribution_sit_up.csv
results/tables/table_optional_pilot_comparison.csv

results/figures/fig_inference_latency_sit_up.png
results/figures/fig_inference_latency_sit_up.pdf
results/figures/fig_cl_update_cost_sit_up.png
results/figures/fig_cl_update_cost_sit_up.pdf
results/figures/fig_segment_accepted_rate_sit_up.png
results/figures/fig_segment_accepted_rate_sit_up.pdf
results/figures/fig_upstairs_like_shift_sit_up.png
results/figures/fig_upstairs_like_shift_sit_up.pdf
results/figures/fig_prediction_distribution_upstairs_like.png
results/figures/fig_prediction_distribution_upstairs_like.pdf
```

**Notebook contents**:

- intro with correct scientific scope
- safe CSV loading
- raw tables display
- metric cleanup and derived columns
- resource/CL overhead table
- MicroFlow-32 inference latency plot
- CL update cost plot
- segment accepted-rate plots
- upstairs-like prediction shift plot
- prediction-distribution parser and plot
- optional comparison with earlier standing-like pilot
- final paper-ready summary bullets

**Validation**:

```bash
python3 - <<'PY'
import ast
import json
from pathlib import Path

path = Path('notebooks/paper_results_analysis.ipynb')
nb = json.loads(path.read_text(encoding='utf-8'))
code_cells = [cell for cell in nb['cells'] if cell.get('cell_type') == 'code']
for idx, cell in enumerate(code_cells, start=1):
    source = ''.join(cell.get('source', []))
    compile(source, f'{path}:code_cell_{idx}', 'exec')
print(f'valid notebook json: {path}')
print(f'code cells compiled: {len(code_cells)}')
PY
```

Output:

```text
valid notebook json: notebooks/paper_results_analysis.ipynb
code cells compiled: 14
```

**Висновок**:

- notebook готовий для ручного top-to-bottom запуску
- головний analysis focus: `Sitting` vs upstairs-like vertical hand-motion pilot
- plots/tables формуються без нових firmware changes і без rerun hardware experiments

## Фаза 5i — Український audit результатів, таблиць і графіків

**Що зроблено**: після повного перечитування `DEVLOG.md` додано окремий український технічний analysis note для Phase 5 results. Мета цього кроку — зафіксувати не загальні тези, а конкретну інтерпретацію вже згенерованих CSV, таблиць і графіків перед переходом до тексту статті.

**Додано**:

- [`results/analysis_notes_uk.md`](/home/g00n3r/projects/esp32_cl_har/results/analysis_notes_uk.md:1)
  - межі інтерпретації поточних результатів
  - список primary/secondary pilot data sources
  - resource and CL overhead numbers для `Sitting` vs upstairs-like pilot
  - segment-level interpretation для `no_adapt / FIFO / reservoir`
  - пояснення кожного наявного графіка з `results/figures`
  - allowed claims і forbidden claims для статті
  - список того, що вже можна переносити в `Experimental Setup`, `Results`, `Discussion`
  - список того, чого ще бракує
  - наступний рекомендований маленький крок: attempt-level plots

**Синхронізовано `PLAN.md`**:

- `Згенерувати графіки matplotlib` позначено як виконане для основного `Sitting` vs upstairs-like pilot
- додано виконаний пункт про український analysis audit
- наступний графічний крок уточнено як `attempt-level plots prediction/confidence vs attempt`

**Ключові зафіксовані числові результати**:

```text
MicroFlow-32 inference:
  no_adapt   mean = 172.511911 ms
  fifo       mean = 172.289640 ms
  reservoir  mean = 172.324380 ms

CL update:
  fifo       mean = 666.5 us, overhead = 0.387%
  reservoir  mean = 656.0 us, overhead = 0.381%

Replay RAM:
  6 classes x 16 slots/class x 32 f32 features = 12 KiB

Upstairs-like segment accepted rate:
  no_adapt   = 0.0000
  fifo       = 0.8857
  reservoir  = 0.9394
```

**Scientific wording зафіксовано**:

- поточний результат є `real-device pilot sanity check`, не full `6-class HAR` benchmark
- upstairs-like segment є vertical hand-motion біля ПК, не real staircase benchmark
- `reservoir` показав трохи вищий accepted rate у pilot, але це не statistical superiority claim
- головний publishable resource result: RAM-only CL update overhead менший за `1%` від `MicroFlow-32` inference time
- persistence/NVS/flash state лишаються Future Work

**Межі кроку**:

- firmware не змінювалась
- `main.rs` не змінювався
- raw logs не змінювались
- hardware experiments не перезапускались
- git не чіпався

**Висновок**:

- Phase 5 тепер має не тільки raw/parsed results і notebook plots, а й український технічний audit для коректного перенесення результатів у статтю
- наступний маленький крок: додати attempt-level plots для `prediction/confidence vs attempt`, щоб показати часову динаміку prediction shift після labels/train

## Фаза 6a — Українська структура статті і mapping результатів

**Що зроблено**: створено робочий outline статті українською мовою, який переводить accumulated `DEVLOG`/results матеріал у статейну структуру. Це не фінальний текст статті, а каркас для подальшого написання `Introduction`, `System Architecture`, `Experimental Setup`, `Results`, `Discussion` і `Conclusion`.

**Додано**:

- [`paper/article_structure_uk.md`](/home/g00n3r/projects/esp32_cl_har/paper/article_structure_uk.md:1)
  - центральна ідея статті
  - головний обережний claim
  - список claims, які не можна робити
  - рекомендована структура секцій
  - mapping числових результатів у `Results`
  - окреме формулювання ролі USB/UART тільки як limitation
  - figure plan
  - table plan
  - заборонені наступні кроки в поточному sprint

**Ключове змістове рішення**:

```text
USB/UART не є центральною темою статті.
Це лише limitation поточного експериментального стенду.
Центральна тема: minimal RAM-only replay-based CL pipeline на ESP32 з реальним MPU6050,
resource profiling і pilot evidence prediction shift після supervised labels.
```

**Запропонована структура статті**:

```text
1. Introduction
2. Related Work
3. System Architecture
4. Experimental Setup
5. Results
6. Discussion
7. Conclusion
```

**Основний Results mapping**:

- `5.1 Offline baseline and feature extractor selection`
- `5.2 Runtime cost of RAM-only CL`
- `5.3 Real-device pilot: Sitting vs upstairs-like motion`
- `5.4 Prediction distribution shift`
- `5.5 Secondary standing-like pilot`

**Figure plan**:

- system architecture diagram — треба зробити
- `MicroFlow-64 vs MicroFlow-32 latency` — треба зробити
- `fig_inference_latency_sit_up` — готово
- `fig_cl_update_cost_sit_up` — готово
- `fig_upstairs_like_shift_sit_up` — готово
- `fig_prediction_distribution_upstairs_like` — готово
- attempt-level `prediction/confidence vs attempt` — наступний маленький крок

**Синхронізовано `PLAN.md`**:

- у `Фазі 6` додано виконаний пункт про українську структуру статті і mapping результатів/графіків

**Межі кроку**:

- firmware не змінювалась
- `main.rs` не змінювався
- raw logs не змінювались
- hardware experiments не перезапускались
- git не чіпався

**Висновок**:

- напрям статті зафіксовано: не зациклюватись на USB/кабелі, не роздувати Bluetooth/persistence, а писати про ESP32 CL реалізацію, resource analysis і чесний pilot на реальному MPU6050
- наступний практичний крок лишається технічно малим: attempt-level plots з уже наявних parsed logs

## Фаза 5j — Attempt-level plots для `Sitting` vs upstairs-like pilot

**Що зроблено**: існуючий paper-results notebook розширено секцією `Attempt-Level Dynamics`, без створення нового notebook. Також згенеровано готові PNG/PDF figures з уже parsed CSV для основного `Sitting` vs upstairs-like vertical hand-motion pilot.

**Змінено**:

- [`notebooks/paper_results_analysis.ipynb`](/home/g00n3r/projects/esp32_cl_har/notebooks/paper_results_analysis.ipynb:1)
  - додано завантаження parsed `PRED`, `LABEL`, `TRAIN` CSV для `no_adapt`, `fifo`, `reservoir`
  - додано `prediction class vs attempt`
  - додано `confidence vs attempt`
  - додано segment boundary markers між `sitting` і `upstairs_like`
  - додано `TRAIN` markers для FIFO/reservoir

**Generated figures**:

```text
results/figures/fig_prediction_class_attempt_sit_up.png
results/figures/fig_prediction_class_attempt_sit_up.pdf
results/figures/fig_confidence_attempt_sit_up.png
results/figures/fig_confidence_attempt_sit_up.pdf
```

**Generated table**:

```text
results/tables/table_attempt_level_events_sit_up.csv
```

**Attempt markers**:

```text
mode       sitting   upstairs_like   labels attempts   train attempts
no_adapt   1-18      19-45           -                 -
fifo       1-15      16-50           8;9;26;27         9;27
reservoir  1-17      18-50           9;10;27;28        10;28
```

**Validation**:

```bash
python3 - <<'PY'
import json
from pathlib import Path
path = Path('notebooks/paper_results_analysis.ipynb')
nb = json.loads(path.read_text(encoding='utf-8'))
code_cells = [cell for cell in nb['cells'] if cell.get('cell_type') == 'code']
for idx, cell in enumerate(code_cells, start=1):
    source = ''.join(cell.get('source', []))
    compile(source, f'{path}:code_cell_{idx}', 'exec')
print(f'valid notebook json: {path}')
print(f'code cells compiled: {len(code_cells)}')
print(f'total cells: {len(nb["cells"])}')
PY
```

Output:

```text
valid notebook json: notebooks/paper_results_analysis.ipynb
code cells compiled: 17
total cells: 35
```

**Scientific interpretation**:

- `no_adapt` у `upstairs_like` segment лишається на class `Sitting`, хоча confidence падає під час руху
- `fifo` і `reservoir` після supervised labels/train зміщують predictions у `Upstairs`/`Downstairs`
- ці plots показують часову динаміку pilot, а не фінальну `6-class HAR` accuracy

**Синхронізовано документацію**:

- `PLAN.md`: attempt-level plots позначено виконаними
- `results/analysis_notes_uk.md`: next step змінено на `MicroFlow-64 vs MicroFlow-32 latency plot`
- `paper/article_structure_uk.md`: figure plan оновлено, attempt-level plots позначено готовими

**Межі кроку**:

- firmware не змінювалась
- `main.rs` не змінювався
- raw logs не змінювались
- hardware experiments не перезапускались
- git не чіпався

**Висновок**:

- Results section тепер має не тільки aggregate bar charts, а й attempt-level динаміку prediction/confidence для основного pilot
- наступний малий графічний крок: окремий `MicroFlow-64 vs MicroFlow-32 latency` plot з уже зафіксованих DEVLOG values

## Фаза 5k — `MicroFlow-64` vs `MicroFlow-32` latency ablation plot

**Що зроблено**: існуючий paper-results notebook розширено секцією `MicroFlow Feature Extractor Latency Ablation`. З уже зафіксованих у `DEVLOG.md` hardware latency checkpoints згенеровано компактний графік, який пояснює, чому основний embedded CL path перейшов з `MicroFlow-64` на `MicroFlow-32`.

**Джерела чисел**:

- `MicroFlow-64`: `DEVLOG.md`, Phase 3j, 20-attempt streaming mean
- `MicroFlow-32`: `DEVLOG.md`, Phase 3m, 20-attempt streaming mean

**Змінено**:

- [`notebooks/paper_results_analysis.ipynb`](/home/g00n3r/projects/esp32_cl_har/notebooks/paper_results_analysis.ipynb:1)
  - додано latency ablation table
  - додано bar plot `MicroFlow-64` vs `MicroFlow-32`
  - додано коротку інтерпретацію, що `MicroFlow-32` є primary embedded path, а `MicroFlow-64` лишається reference

**Generated files**:

```text
results/figures/fig_microflow_latency_ablation.png
results/figures/fig_microflow_latency_ablation.pdf
results/tables/table_microflow_latency_ablation.csv
```

**Табличні значення**:

```text
extractor     feature_dim  mean_latency_ms  replay_ram_kib  latency_reduction_vs_64_pct
MicroFlow-64  64           298.683          24.0            0.0
MicroFlow-32  32           172.017          12.0            42.408
```

**Interpretation**:

- `MicroFlow-32` зменшує streaming feature extraction latency приблизно на `42.4%`
- replay RAM estimate зменшується з `24 KiB` до `12 KiB`
- це обґрунтовує використання `MicroFlow-32` як основного embedded path для CL experiments
- `MicroFlow-64` не викидається, а лишається stronger/reference path

**Validation**:

```text
valid notebook json: notebooks/paper_results_analysis.ipynb
code cells compiled: 19
total cells: 39
```

**Синхронізовано документацію**:

- `PLAN.md`: `MicroFlow-64 vs MicroFlow-32 latency` plot позначено виконаним
- `results/analysis_notes_uk.md`: next step змінено на `Results draft`
- `paper/article_structure_uk.md`: figure plan оновлено, latency ablation позначено готовим

**Межі кроку**:

- firmware не змінювалась
- `main.rs` не змінювався
- raw logs не змінювались
- hardware experiments не перезапускались
- git не чіпався

**Висновок**:

- основний набір графіків для `Results` тепер покриває latency ablation, CL overhead, segment accepted rate, prediction distribution і attempt-level dynamics
- наступний логічний крок: почати `Results` draft на основі вже готових tables/figures, без нових embedded features

## Фаза 6b — Український `Results` draft

**Що зроблено**: створено перший робочий draft секції `Results` українською мовою на основі вже готових таблиць, графіків і parsed experiment outputs. Це не додає нових експериментів і не змінює firmware; крок переводить Phase 5 artifacts у статейний текст.

**Додано**:

- [`paper/results_draft_uk.md`](/home/g00n3r/projects/esp32_cl_har/paper/results_draft_uk.md:1)

**Структура draft-у**:

```text
1. Offline baseline і вибір embedded feature extractor
2. Runtime cost RAM-only continual learning
3. Real-device pilot: Sitting vs upstairs-like vertical motion
4. Prediction distribution і attempt-level dynamics
5. Secondary pilot: Sitting vs standing-like small movement
6. Summary of result claims
7. Limitations visible from the results
```

**Головні використані артефакти**:

```text
results/figures/fig_microflow_latency_ablation.png
results/figures/fig_inference_latency_sit_up.png
results/figures/fig_cl_update_cost_sit_up.png
results/figures/fig_segment_accepted_rate_sit_up.png
results/figures/fig_upstairs_like_shift_sit_up.png
results/figures/fig_prediction_distribution_upstairs_like.png
results/figures/fig_prediction_class_attempt_sit_up.png
results/figures/fig_confidence_attempt_sit_up.png

results/tables/table_microflow_latency_ablation.csv
results/tables/table_resource_overhead_sit_up.csv
results/tables/table_prediction_distribution_sit_up.csv
results/tables/table_attempt_level_events_sit_up.csv
logs/parsed/pilot_sit_up/sit_up_segment_eval_2026-05-09.csv
```

**Ключові claims у draft-і**:

- `MicroFlow-32` зменшує latency приблизно на `42.4%` проти `MicroFlow-64`
- replay RAM estimate зменшується з `24 KiB` до `12 KiB`
- RAM-only CL update коштує приблизно `0.66 ms`
- CL update overhead менший за `1%` від `MicroFlow-32` feature extraction latency
- upstairs-like pilot показує prediction shift після supervised labels:
  - `no_adapt`: `0.0%`
  - `FIFO`: `88.57%`
  - `reservoir`: `93.94%`

**Scope discipline у тексті**:

- результат описано як feasibility / real-device pilot sanity check
- не заявляється full `6-class HAR` benchmark
- не заявляється real staircase benchmark
- не заявляється statistical superiority reservoir над FIFO
- USB/UART описано як limitation стенду, не як центральну тему
- persistence/NVS/flash storage лишено Future Work

**Синхронізовано `PLAN.md`**:

- `Results draft на основі готових таблиць/графіків` позначено виконаним у `Фазі 6`

**Межі кроку**:

- firmware не змінювалась
- `main.rs` не змінювався
- raw logs не змінювались
- hardware experiments не перезапускались
- git не чіпався

**Висновок**:

- секція `Results` тепер має перший coherent draft, який можна редагувати в текст статті
- наступний логічний крок: підготувати `Experimental Setup` draft, щоб формально пояснити hardware, firmware modes, pilot protocol і metrics перед Results

## Фаза 6c — Український `Experimental Setup` draft

**Що зроблено**: створено перший робочий draft секції `Experimental Setup` українською мовою. Секція формально пояснює hardware, firmware stack, offline model preparation, ESP32 preprocessing, CL components, compared modes, label protocol, logging/parsing, pilot protocol, metrics і scope boundaries.

**Додано**:

- [`paper/experimental_setup_draft_uk.md`](/home/g00n3r/projects/esp32_cl_har/paper/experimental_setup_draft_uk.md:1)

**Структура draft-у**:

```text
1. Hardware platform
2. Firmware stack
3. Offline model preparation
4. Sensor preprocessing on ESP32
5. Continual learning components
6. Compared modes
7. Label protocol
8. Logging and parsing
9. Pilot protocol
10. Metrics
11. Scope boundaries
```

**Ключові зафіксовані параметри**:

```text
Hardware:
  ESP32-WROOM-32 / ESP32-D0WD-V3 rev v3.1
  240 MHz
  4 MB Flash
  320 KB SRAM class target
  MPU6050 / GY-521
  I2C SDA=GPIO21, SCL=GPIO22

Firmware:
  Rust 2024
  no_std
  esp-hal
  xtensa-esp32-none-elf

Windowing:
  20 Hz sampling
  80 x 3 window
  int8[240] input

CL:
  MicroFlow-32 frozen feature extractor
  OnlineLayer32
  ReplayBuffer32
  6 classes x 16 slots/class x 32 f32 features = 12 KiB
  K=10 labels/update
  batch_size=12
  lr=0.001
  persistence=off
```

**Pilot protocol зафіксовано**:

- main pilot: `Sitting` vs `upstairs-like vertical hand-motion`
- labels: `4` для `Sitting`, `2` для upstairs-like motion
- modes: `no_adapt`, `FIFO`, `reservoir`
- second segment описано як upstairs-like hand motion біля host PC, не real staircase benchmark
- secondary pilot `Sitting vs standing-like small movement` лишається sanity check

**Scope boundaries зафіксовано**:

- setup підтримує claims про end-to-end ESP32 pipeline, RAM-only CL, FIFO/reservoir under same memory budget, CL overhead і pilot prediction shift
- setup не підтримує claims про full `6-class HAR` benchmark, statistical superiority reservoir, real staircase benchmark, autonomous labeling, persistence або strict `20 Hz` inference throughput

**Синхронізовано `PLAN.md`**:

- `Experimental Setup draft` позначено виконаним у `Фазі 6`

**Межі кроку**:

- firmware не змінювалась
- `main.rs` не змінювався
- raw logs не змінювались
- hardware experiments не перезапускались
- git не чіпався

**Висновок**:

- тепер є послідовна пара draft-ів: `Experimental Setup` перед `Results`
- наступний логічний крок: підготувати `System Architecture` draft, щоб пояснити offline/embedded pipeline до експериментальної секції

## Фаза 6d — Український `System Architecture` draft

**Що зроблено**: створено перший робочий draft секції `System Architecture` українською мовою. Секція пояснює повний pipeline від offline training на PC до RAM-only CL loop на ESP32 і чітко відділяє frozen feature extraction, online head, replay storage та experiment modes.

**Додано**:

- [`paper/system_architecture_draft_uk.md`](/home/g00n3r/projects/esp32_cl_har/paper/system_architecture_draft_uk.md:1)

**Структура draft-у**:

```text
1. Overview
2. Offline PC pipeline
3. Deployment-oriented feature extractor
4. ESP32 sensor and preprocessing path
5. Frozen feature extraction on ESP32
6. OnlineLayer32
7. RAM-only ReplayBuffer32
8. RAM-only CL loop
9. Experiment modes as architecture variants
10. Logging architecture
11. Architectural boundaries and contribution
12. Why this architecture fits ESP32
```

**Ключові архітектурні рішення зафіксовано**:

- split architecture: PC training/export, ESP32 frozen inference + last-layer adaptation
- `MicroFlow-32` є primary embedded feature extractor
- `MicroFlow-64` лишається stronger/reference path
- `OnlineLayer32` є єдиною trainable частиною на ESP32
- `ReplayBuffer32` зберігає latent features, не raw IMU windows
- `FIFO` і `reservoir-per-class` порівнюються під однаковим memory budget
- CL state RAM-only, persistence/NVS/flash writes out of scope

**Contribution boundary зафіксовано**:

```text
Не нова CNN архітектура.
Не новий inference runtime.
Не UART/Bluetooth protocol.

Внесок:
frozen feature extractor
+ Rust/no_std online head
+ RAM-only replay
+ FIFO/reservoir comparison
+ real MPU6050 pilot
+ resource measurements on ESP32
```

**Синхронізовано `PLAN.md`**:

- `System Architecture draft` позначено виконаним у `Фазі 6`

**Межі кроку**:

- firmware не змінювалась
- `main.rs` не змінювався
- raw logs не змінювались
- hardware experiments не перезапускались
- git не чіпався

**Висновок**:

- тепер є три узгоджені статейні draft-и: `System Architecture`, `Experimental Setup`, `Results`
- наступний логічний крок: підготувати `Discussion` draft, бо він напряму спирається на вже сформульовані Results/limitations і допоможе не перебільшити claims

## Фаза 6e — Український `Discussion` draft

**Що зроблено**: створено перший робочий draft секції `Discussion` українською мовою. Секція інтерпретує результати, фіксує межі claims, пояснює роль USB/UART, persistence, FIFO/reservoir і future work без перебільшення результатів.

**Додано**:

- [`paper/discussion_draft_uk.md`](/home/g00n3r/projects/esp32_cl_har/paper/discussion_draft_uk.md:1)

**Структура draft-у**:

```text
1. Feasibility of RAM-only continual learning on ESP32
2. Runtime bottleneck: feature extraction, not online update
3. Why MicroFlow-32 became the primary embedded path
4. Interpretation of FIFO vs reservoir
5. What the real-device pilot demonstrates
6. Domain shift and why adaptation is needed
7. Label acquisition and UART limitations
8. Why persistence is future work
9. Limitations
10. Future work
11. Main interpretation
```

**Ключові інтерпретації зафіксовано**:

- головний результат — feasibility/resource profile, не SOTA accuracy
- online CL update дешевий відносно frozen feature extraction
- `MicroFlow-32` обрано через latency/RAM trade-off, а не як scientific contribution
- `FIFO` і `reservoir` обидва працюють під однаковим memory budget
- reservoir виглядає promising у pilot, але statistical superiority не заявляється
- real-device pilot демонструє prediction shift, не full `6-class HAR` benchmark
- USB/UART є limitation стенду, не центральна тема статті
- persistence/NVS/flash writes лишаються Future Work

**Синхронізовано `PLAN.md`**:

- `Discussion draft` позначено виконаним у `Фазі 6`

**Межі кроку**:

- firmware не змінювалась
- `main.rs` не змінювався
- raw logs не змінювались
- hardware experiments не перезапускались
- git не чіпався

**Висновок**:

- тепер є чотири узгоджені draft-и: `System Architecture`, `Experimental Setup`, `Results`, `Discussion`
- наступний логічний крок: підготувати `Conclusion` draft, а потім окремо `Introduction + Related Work`

## Фаза 6f — Український `Conclusion` draft

**Що зроблено**: створено перший робочий draft секції `Conclusion` українською мовою. Секція коротко підсумовує feasibility, architecture, resource result, real-device pilot, limitations і future work без розширення claims.

**Додано**:

- [`paper/conclusion_draft_uk.md`](/home/g00n3r/projects/esp32_cl_har/paper/conclusion_draft_uk.md:1)

**Ключові тези conclusion**:

- мінімальний replay-based CL pipeline для IMU-HAR реалізовано на `ESP32-WROOM-32` у `Rust/no_std`
- основний path: `MPU6050 -> MicroFlow-32 -> OnlineLayer32 -> ReplayBuffer32`
- `MicroFlow-32` зменшує latency проти `MicroFlow-64` приблизно з `298.7 ms` до `172.0 ms`
- replay RAM estimate зменшується з `24 KiB` до `12 KiB`
- `OnlineLayer32.backward_batch()` з replay mini-batch коштує приблизно `0.66 ms`
- CL update overhead нижчий за `1%` від feature extraction time
- real-device pilot показує prediction shift після supervised labels
- full `6-class HAR` benchmark, autonomous labels і persistence лишаються Future Work

**Синхронізовано `PLAN.md`**:

- `Conclusion draft` позначено виконаним у `Фазі 6`

**Межі кроку**:

- firmware не змінювалась
- `main.rs` не змінювався
- raw logs не змінювались
- hardware experiments не перезапускались
- git не чіпався

**Висновок**:

- тепер є п'ять узгоджених draft-ів: `System Architecture`, `Experimental Setup`, `Results`, `Discussion`, `Conclusion`
- наступний логічний крок: підготувати `Introduction + Related Work`, які мають зв'язати внесок з TinyML HAR, TinyOL/replay і сучасними складнішими HAR+CL роботами без SOTA-claim

## Фаза 6g — Український `Introduction + Related Work` draft

**Що зроблено**: створено перший робочий draft секцій `Introduction` і `Related Work` українською мовою. Перед написанням звірено точні назви й positioning для згаданих сучасних робіт: Kwon/LifeLearner, Fusco et al. on-device training/pruning, COOL 2026, PACL+ і OCL-HAR.

**Додано**:

- [`paper/introduction_related_work_draft_uk.md`](/home/g00n3r/projects/esp32_cl_har/paper/introduction_related_work_draft_uk.md:1)

**Структура draft-у**:

```text
Introduction
Contributions

Related Work:
1. TinyML and on-device learning on microcontrollers
2. Continual learning for mobile and embedded sensing
3. Online continual learning for HAR
4. On-device continual learning for HAR on MCU
5. Positioning of this work
Reference Notes
```

**Ключові related work references зафіксовано**:

- `TinyOL: TinyML with Online-Learning on Microcontrollers` — TinyML online learning на MCU
- Kwon et al. `LifeLearner` — hardware-aware meta CL, latent replay, product quantization, embedded platforms
- Schiemer et al. `Online continual learning for human activity recognition` — HAR-specific OCL scenario
- `PACL+` — proxy-anchor / contrastive learning / Gaussian replay для sensor-based HAR
- Fusco et al. `On-device training and pruning for energy saving and continuous learning in resource-constrained MCUs` — ESP32/STM32, on-device pruning/training, energy/resource focus
- `COOL: continual online on-device learning for human activity recognition enhanced by KANs` — найпряміший сучасний HAR+on-device CL MCU reference, KAN-based, STM32H743

**Positioning зафіксовано**:

```text
Наша робота не претендує на SOTA HAR accuracy.
Наша робота не конкурує напряму з COOL або LifeLearner за algorithmic complexity.
Наша ніша:
ESP32-WROOM-32 + real MPU6050 + Rust/no_std + MicroFlow-32 frozen extractor
+ OnlineLayer32 + RAM-only ReplayBuffer32 + FIFO/reservoir comparison
+ resource metrics + pilot prediction shift.
```

**Синхронізовано `PLAN.md`**:

- `Introduction + Related Work draft` позначено виконаним у `Фазі 6`

**Межі кроку**:

- firmware не змінювалась
- `main.rs` не змінювався
- raw logs не змінювались
- hardware experiments не перезапускались
- git не чіпався

**Висновок**:

- тепер є повний набір українських draft-ів основних секцій статті: `Introduction + Related Work`, `System Architecture`, `Experimental Setup`, `Results`, `Discussion`, `Conclusion`
- наступний логічний крок: зібрати unified article draft або зробити consistency pass між усіма section drafts

## Фаза 7a — WISDM device-side eval Stage 0/1

**Що зроблено**: додано ізольований device-side WISDM inference evaluation path для ESP32. Це окремий sanity/F1-прогін відомих WISDM windows на пристрої:

```text
int8[240] WISDM window
-> MicroFlow-32 frozen feature extractor
-> pretrained OnlineLayer32
-> prediction
-> confusion matrix
```

Цей крок не змінює normal sensor firmware і не є CL-експериментом.

**Додано**:

- [`scripts/export_wisdm_device_eval_artifact.py`](/home/g00n3r/projects/esp32_cl_har/scripts/export_wisdm_device_eval_artifact.py:1)
- [`scripts/parse_wisdm_device_eval.py`](/home/g00n3r/projects/esp32_cl_har/scripts/parse_wisdm_device_eval.py:1)
- [`src/bin/wisdm_device_eval.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/wisdm_device_eval.rs:1)
- [`src/eval_artifacts/wisdm_eval_windows_i8_smoke_120.bin`](/home/g00n3r/projects/esp32_cl_har/src/eval_artifacts/wisdm_eval_windows_i8_smoke_120.bin:1)
- [`src/eval_artifacts/wisdm_eval_labels_u8_smoke_120.bin`](/home/g00n3r/projects/esp32_cl_har/src/eval_artifacts/wisdm_eval_labels_u8_smoke_120.bin:1)
- [`src/eval_artifacts/wisdm_eval_metadata_smoke_120.json`](/home/g00n3r/projects/esp32_cl_har/src/eval_artifacts/wisdm_eval_metadata_smoke_120.json:1)
- [`src/eval_artifacts/wisdm_eval_windows_i8_balanced_600.bin`](/home/g00n3r/projects/esp32_cl_har/src/eval_artifacts/wisdm_eval_windows_i8_balanced_600.bin:1)
- [`src/eval_artifacts/wisdm_eval_labels_u8_balanced_600.bin`](/home/g00n3r/projects/esp32_cl_har/src/eval_artifacts/wisdm_eval_labels_u8_balanced_600.bin:1)
- [`src/eval_artifacts/wisdm_eval_metadata_balanced_600.json`](/home/g00n3r/projects/esp32_cl_har/src/eval_artifacts/wisdm_eval_metadata_balanced_600.json:1)
- [`src/eval_artifacts/wisdm_eval_windows_i8_full_9154.bin`](/home/g00n3r/projects/esp32_cl_har/src/eval_artifacts/wisdm_eval_windows_i8_full_9154.bin:1)
- [`src/eval_artifacts/wisdm_eval_labels_u8_full_9154.bin`](/home/g00n3r/projects/esp32_cl_har/src/eval_artifacts/wisdm_eval_labels_u8_full_9154.bin:1)
- [`src/eval_artifacts/wisdm_eval_metadata_full_9154.json`](/home/g00n3r/projects/esp32_cl_har/src/eval_artifacts/wisdm_eval_metadata_full_9154.json:1)

**Stage 0 — artifact generator**:

Команда:

```bash
/home/g00n3r/.venvs/base/bin/python scripts/export_wisdm_device_eval_artifact.py
```

Згенеровані artifact-и:

```text
smoke_120:
  windows_bytes=28,800
  labels_bytes=120
  distribution=20/class x 6

balanced_600:
  windows_bytes=144,000
  labels_bytes=600
  distribution=100/class x 6

full_9154:
  windows_bytes=2,196,960
  labels_bytes=9,154
  distribution:
    Walking=4302
    Jogging=2133
    Upstairs=1211
    Downstairs=959
    Sitting=180
    Standing=369
```

`balanced_1200` не згенеровано, бо відтворений final corpus має тільки `180` windows для класу `Sitting`, а чесний `balanced_1200` потребує `200/class`. Oversampling або підміна розміру не виконувались.

**Stage 1 — ESP32 smoke run, 120 windows**:

Build:

```bash
. $HOME/export-esp.sh && cargo build --features microflow32_backend --bin wisdm_device_eval
```

Результат:

```text
Finished `dev` profile [optimized + debuginfo]
```

Size:

```text
xtensa-esp32-elf-size target/xtensa-esp32-none-elf/debug/wisdm_device_eval

text=119854
data=1632
bss=194976
dec=316462
```

Перший запуск у sandbox не побачив USB-порт:

```text
Error: espflash::no_serial
No serial ports could be detected
```

Після запуску поза sandbox `espflash` побачив плату:

```text
Serial port: /dev/ttyUSB0
Chip type: esp32 revision v3.1
Flash size: 4MB
App/part. size: 179,584/4,128,768 bytes, 4.35%
```

Run:

```bash
script -q -c "timeout 180s bash -lc '. $HOME/export-esp.sh && cargo run --features microflow32_backend --bin wisdm_device_eval'" logs/raw/wisdm_device_eval/wisdm_device_eval_smoke_120_2026-05-13.txt
```

Raw log:

- [`logs/raw/wisdm_device_eval/wisdm_device_eval_smoke_120_2026-05-13.txt`](/home/g00n3r/projects/esp32_cl_har/logs/raw/wisdm_device_eval/wisdm_device_eval_smoke_120_2026-05-13.txt:1)

Stage 1 firmware output:

```text
WISDM_EVAL_START tag=smoke_120 total=120
WISDM_EVAL_PROGRESS idx=20 total=120 correct=20 acc=1
WISDM_EVAL_PROGRESS idx=40 total=120 correct=38 acc=0.95
WISDM_EVAL_PROGRESS idx=60 total=120 correct=51 acc=0.85
WISDM_EVAL_PROGRESS idx=80 total=120 correct=59 acc=0.7375
WISDM_EVAL_PROGRESS idx=100 total=120 correct=77 acc=0.77
WISDM_EVAL_PROGRESS idx=120 total=120 correct=97 acc=0.80833334
WISDM_EVAL_SUMMARY tag=smoke_120 total=120 correct=97 accuracy=0.80833334 mean_infer_us=171714 min_infer_us=171245 max_infer_us=173169
```

Per-class recall:

```text
Walking:    support=20 correct=20 recall=1.00
Jogging:    support=20 correct=18 recall=0.90
Upstairs:   support=20 correct=13 recall=0.65
Downstairs: support=20 correct=8  recall=0.40
Sitting:    support=20 correct=18 recall=0.90
Standing:   support=20 correct=20 recall=1.00
```

Confusion matrix:

```text
true=0 pred0=20 pred1=0  pred2=0  pred3=0 pred4=0  pred5=0
true=1 pred0=1  pred1=18 pred2=0  pred3=1 pred4=0  pred5=0
true=2 pred0=2  pred1=3  pred2=13 pred3=2 pred4=0  pred5=0
true=3 pred0=4  pred1=1  pred2=7  pred3=8 pred4=0  pred5=0
true=4 pred0=0  pred1=0  pred2=0  pred3=0 pred4=18 pred5=2
true=5 pred0=0  pred1=0  pred2=0  pred3=0 pred4=0  pred5=20
```

Parsed outputs:

- [`results/tables/wisdm_device_eval_summary.csv`](/home/g00n3r/projects/esp32_cl_har/results/tables/wisdm_device_eval_summary.csv:1)
- [`results/tables/wisdm_device_eval_per_class_smoke_120.csv`](/home/g00n3r/projects/esp32_cl_har/results/tables/wisdm_device_eval_per_class_smoke_120.csv:1)
- [`results/tables/wisdm_device_eval_confusion_smoke_120.csv`](/home/g00n3r/projects/esp32_cl_har/results/tables/wisdm_device_eval_confusion_smoke_120.csv:1)

**Interpretation**:

- Stage 1 gate пройдений: ESP32 booted, 120 WISDM windows processed, summary/confusion matrix printed.
- Mean inference latency `171.714 ms` узгоджується з попередніми MicroFlow-32 hardware measurements.
- Загальна accuracy на tiny balanced smoke subset: `80.83%`.
- Найслабша пара лишається `Upstairs/Downstairs`, що узгоджується з раніше зафіксованою складністю цих класів у WISDM.

**Scope boundaries**:

- `main.rs` не змінювався.
- Normal MPU6050/sensor firmware не змінювалась.
- UART labels не використовувались.
- ReplayBuffer і CL training не використовувались.
- Persistence/NVS/flash writes не додавались.
- Real MPU6050 pilot не перезапускався.
- Це device-side WISDM subset sanity evaluation, не full LOSO CV на ESP32.

**Висновок**:

- Безпечно переходити до Stage 2 `balanced_600`, але тільки окремим gated кроком.
- Stage 3 у початковому вигляді `balanced_1200 = 200/class` неможливий без зміни протоколу, бо `Sitting` має тільки `180` windows у final corpus.

## Фаза 7b — WISDM device-side eval Stage 2 `balanced_600`

**Що зроблено**: після успішного Stage 1 isolated binary `wisdm_device_eval` переключено з `smoke_120` на `balanced_600`. Логіка binary не змінювалась: той самий шлях `int8[240] -> MicroFlow-32 -> OnlineLayer32 -> confusion matrix`, без CL, ReplayBuffer, UART labels або sensor path.

**Змінено**:

- [`src/bin/wisdm_device_eval.rs`](/home/g00n3r/projects/esp32_cl_har/src/bin/wisdm_device_eval.rs:1)
  - `TAG` змінено на `balanced_600`
  - `include_bytes!` переключено на `wisdm_eval_windows_i8_balanced_600.bin`
  - `include_bytes!` переключено на `wisdm_eval_labels_u8_balanced_600.bin`

**Build**:

```bash
. $HOME/export-esp.sh && cargo build --features microflow32_backend --bin wisdm_device_eval
```

Результат:

```text
Finished `dev` profile [optimized + debuginfo]
```

**Size check**:

```bash
xtensa-esp32-elf-size target/xtensa-esp32-none-elf/debug/wisdm_device_eval
xtensa-esp32-elf-size -A target/xtensa-esp32-none-elf/debug/wisdm_device_eval
```

Основні size values:

```text
text=235534
data=1632
bss=194976
dec=432142
.rodata=182696
```

Під час flash:

```text
App/part. size: 245,120/4,128,768 bytes, 5.94%
```

**Run**:

```bash
script -q -c "timeout 420s bash -lc '. $HOME/export-esp.sh && cargo run --features microflow32_backend --bin wisdm_device_eval'" logs/raw/wisdm_device_eval/wisdm_device_eval_balanced_600_2026-05-13.txt
```

Raw log:

- [`logs/raw/wisdm_device_eval/wisdm_device_eval_balanced_600_2026-05-13.txt`](/home/g00n3r/projects/esp32_cl_har/logs/raw/wisdm_device_eval/wisdm_device_eval_balanced_600_2026-05-13.txt:1)

**Stage 2 result**:

```text
WISDM_EVAL_START tag=balanced_600 total=600
WISDM_EVAL_SUMMARY tag=balanced_600 total=600 correct=478 accuracy=0.7966667 mean_infer_us=171714 min_infer_us=171306 max_infer_us=173165
```

Усі `600/600` windows оброблено. Panic/reset/watchdog не зафіксовано. Firmware продовжила працювати після друку summary/confusion matrix.

**Per-class recall**:

```text
Walking:    support=100 correct=98  recall=0.98
Jogging:    support=100 correct=96  recall=0.96
Upstairs:   support=100 correct=71  recall=0.71
Downstairs: support=100 correct=25  recall=0.25
Sitting:    support=100 correct=88  recall=0.88
Standing:   support=100 correct=100 recall=1.00
```

**Confusion matrix**:

```text
true=0 pred0=98 pred1=0  pred2=2  pred3=0  pred4=0  pred5=0
true=1 pred0=2  pred1=96 pred2=1  pred3=1  pred4=0  pred5=0
true=2 pred0=19 pred1=4  pred2=71 pred3=4  pred4=0  pred5=2
true=3 pred0=21 pred1=3  pred2=51 pred3=25 pred4=0  pred5=0
true=4 pred0=0  pred1=0  pred2=2  pred3=0  pred4=88 pred5=10
true=5 pred0=0  pred1=0  pred2=0  pred3=0  pred4=0  pred5=100
```

**Parsed outputs**:

Parser command:

```bash
python3 scripts/parse_wisdm_device_eval.py \
  logs/raw/wisdm_device_eval/wisdm_device_eval_smoke_120_2026-05-13.txt \
  logs/raw/wisdm_device_eval/wisdm_device_eval_balanced_600_2026-05-13.txt
```

Generated/updated:

- [`results/tables/wisdm_device_eval_summary.csv`](/home/g00n3r/projects/esp32_cl_har/results/tables/wisdm_device_eval_summary.csv:1)
- [`results/tables/wisdm_device_eval_per_class_balanced_600.csv`](/home/g00n3r/projects/esp32_cl_har/results/tables/wisdm_device_eval_per_class_balanced_600.csv:1)
- [`results/tables/wisdm_device_eval_confusion_balanced_600.csv`](/home/g00n3r/projects/esp32_cl_har/results/tables/wisdm_device_eval_confusion_balanced_600.csv:1)

Summary CSV тепер містить два staged rows:

```text
smoke_120:     total=120 correct=97  accuracy=0.80833334 mean_infer_us=171714
balanced_600:  total=600 correct=478 accuracy=0.7966667  mean_infer_us=171714
```

**Синхронізовано analysis notes**:

- [`results/analysis_notes_uk.md`](/home/g00n3r/projects/esp32_cl_har/results/analysis_notes_uk.md:1)
  - додано note, що device-side WISDM balanced subset evaluation доступний
  - зафіксовано, що це inference-only sanity evaluation, не CL і не full LOSO CV

**Interpretation**:

- `balanced_600` є першим paper-safe on-device WISDM subset result.
- Accuracy `79.67%` близька до раніше зафіксованого offline LOSO macro-level baseline range, але її треба описувати як balanced subset sanity evaluation, а не як повний LOSO result.
- Mean inference latency `171.714 ms` стабільно повторює Stage 1 і попередні MicroFlow-32 hardware measurements.
- Найбільша помилка лишається у класі `Downstairs`: `51/100` samples передбачено як `Upstairs`, що узгоджується з відомою плутаниною stair-like класів.

**Scope boundaries**:

- `main.rs` не змінювався.
- Normal MPU6050/sensor firmware не змінювалась.
- UART dataset streaming не додано.
- UART labels не використовувались.
- ReplayBuffer і CL training не використовувались.
- Persistence/NVS/flash writes не додавались.
- Real MPU6050 pilot не перезапускався.

**Висновок**:

- Stage 2 gate пройдений.
- Stage 3 у формі `balanced_1200 = 200/class` не можна запускати без зміни протоколу, бо `Sitting` має лише `180` windows у final corpus.
- Наступний безпечний варіант: або зупинити WISDM device-side eval на `balanced_600` як paper-safe result, або окремим рішенням змінити Stage 3 на `balanced_1080 = 180/class`.

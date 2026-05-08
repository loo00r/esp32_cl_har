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

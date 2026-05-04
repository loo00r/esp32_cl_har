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

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

# Development Log

Покрокова документація розробки системи Continual Learning для HAR на ESP32 (C++ / TFLite Micro).
Цей файл ведеться паралельно з розробкою і слугуватиме основою для розділу **Implementation** у статті.

---

## Фаза 0 — C++ прототип (PlatformIO)

**Мета**: переконатись що плата працює, порт доступний, базові операції (Blink, Serial) функціонують.

**Середовище**: PlatformIO + Arduino framework, C++

**Результат**:
- Плата: ESP32-D0WD-V3 rev3.1, 240 MHz, 4 MB Flash, `/dev/ttyUSB0`
- LED на GPIO2 блимає, Serial monitor працює
- `src/main.cpp` — 50 blink-ів з Serial логом

### Структура пам'яті (орієнтовно)

| Ресурс | Загально | Зарезервовано | Доступно |
|--------|----------|---------------|---------|
| SRAM | 320 KB | ~30 KB (stack, framework) | ~290 KB |
| Flash | 4 MB | ~83 KB (bootloader) | ~3.9 MB |

---

## Наступні кроки

- [ ] Тренування 1D-CNN на WISDM (Python/TensorFlow)
- [ ] Конвертація у `model.tflite` (INT8 quantization)
- [ ] TFLite Micro inference на ESP32 (UART тест)

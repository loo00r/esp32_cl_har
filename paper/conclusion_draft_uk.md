# Draft Conclusion Section

Цей файл є робочим draft-ом секції `Conclusion` для статті про `ESP32 CL-HAR`.
Він підсумовує внесок без розширення claims за межі вже отриманих результатів.

## Conclusion

У цій роботі показано, що мінімальний replay-based continual learning pipeline
для IMU-based human activity recognition може бути реалізований на
`ESP32-WROOM-32` у `Rust/no_std` firmware stack. Система використовує offline
trained frozen feature extractor, реальний `MPU6050` sensor path,
`OnlineLayer32` як lightweight trainable head і RAM-only `ReplayBuffer32` для
supervised in-session adaptation.

Основна архітектура:

```text
MPU6050
-> SlidingWindow 80x3
-> MicroFlow-32 frozen feature extractor
-> OnlineLayer32
-> RAM-only ReplayBuffer32
-> FIFO або reservoir-per-class replay
```

Експерименти показали, що `MicroFlow-32` є практичнішим embedded feature
extractor для ESP32 CL path, ніж `MicroFlow-64`: streaming feature extraction
latency зменшилась приблизно з `298.7 ms` до `172.0 ms`, а replay RAM estimate -
з `24 KiB` до `12 KiB`. Це зробило `MicroFlow-32` основним embedded path, тоді
як `MicroFlow-64` лишився reference/stronger baseline.

Resource profiling показав, що online continual learning update не є головним
runtime bottleneck. У main pilot `OnlineLayer32.backward_batch()` з replay
mini-batch займав приблизно `0.66 ms`, тоді як frozen `MicroFlow-32` feature
extraction займав приблизно `172 ms`. Отже, RAM-only CL update overhead лишився
нижчим за `1%` від часу feature extraction.

Real-device pilot `Sitting vs upstairs-like vertical hand-motion` показав
prediction shift після supervised labels. У режимі `no_adapt` upstairs-like
segment залишався класифікованим як `Sitting`. Після RAM-only adaptation `FIFO`
і `reservoir-per-class` змістили predictions у stair-like класи
`Upstairs/Downstairs`, використовуючи однаковий replay budget `12 KiB`.

Ці результати не є full `6-class HAR` benchmark і не доводять статистичну
перевагу reservoir над FIFO. Натомість вони демонструють feasibility:
resource-constrained replay-based last-layer adaptation може працювати на
ESP32-class MCU з реальним IMU-сенсором, прозорими latency/RAM metrics і
простим supervised feedback channel.

Основні обмеження поточної роботи:

- pilot sessions короткі і не замінюють multi-subject real-device benchmark;
- supervised labels передавались через USB/UART;
- synchronous `MicroFlow-32` inference лишається latency bottleneck;
- CL state є RAM-only і не зберігається після reset;
- autonomous label acquisition і persistence винесені за межі поточного scope.

Майбутня робота має розширити evaluation на кількох subjects і повний набір HAR
classes, покращити label timing, перевірити BLE або mobile-based feedback,
оптимізувати frozen feature extractor latency, а також окремо дослідити
non-volatile persistence з flash wear policy. Проте вже поточний baseline
показує, що `ESP32-WROOM-32` достатній для мінімального replay-based continual
learning pipeline, якщо on-device training обмежити lightweight online head і
RAM-only latent replay.

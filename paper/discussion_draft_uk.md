# Draft Discussion Section

Цей файл є робочим draft-ом секції `Discussion` для статті про `ESP32 CL-HAR`.
Секція пояснює, як інтерпретувати результати, які claims є коректними, які
обмеження лишаються, і що має бути Future Work.

## Discussion

### 1. Feasibility of RAM-only continual learning on ESP32

Основний результат цієї роботи полягає не в досягненні максимальної HAR
accuracy, а в демонстрації того, що мінімальний replay-based continual learning
pipeline може бути реалізований і профільований на `ESP32-WROOM-32` у
`Rust/no_std` firmware stack.

Поточна система об'єднує:

```text
real MPU6050 sensor input
-> MicroFlow-32 frozen feature extractor
-> Rust OnlineLayer32
-> RAM-only ReplayBuffer32
-> FIFO або reservoir-per-class replay
-> supervised UART labels
```

Цей pipeline не лише компілюється, а й працює на реальному MCU: sensor loop,
feature extraction, online forward pass, replay insertion, mini-batch sampling і
online update були перевірені на платі. Це важливо, бо багато continual learning
ідей виглядають простими в offline simulation, але стають нетривіальними при
обмеженнях MCU: SRAM, Flash footprint, latency, відсутність heap-heavy runtime і
відсутність filesystem.

### 2. Runtime bottleneck: feature extraction, not online update

Найважливіший ресурсний висновок полягає в тому, що online CL update не є
головним runtime bottleneck у поточному pipeline. `MicroFlow-32` feature
extraction займає приблизно `172 ms` на window, тоді як `OnlineLayer32`
mini-batch update з replay займає приблизно `0.66 ms`.

Отже, вартість RAM-only last-layer adaptation становить близько `0.38%` від часу
frozen feature extraction. Це підтримує архітектурне рішення навчати на ESP32
лише lightweight final head, а не оновлювати convolutional feature extractor.

Цей результат треба інтерпретувати обережно. Він не означає, що повна система
вже є strict real-time HAR pipeline на `20 Hz`: synchronous `MicroFlow-32`
inference перевищує `50 ms` sampling period. Проте він показує, що якщо
оптимізувати або замінити frozen feature extractor, replay-based online update
не стане основним blocker-ом.

### 3. Why MicroFlow-32 became the primary embedded path

Початковий `MicroFlow-64` path був корисним як stronger/reference feature
extractor, але його streaming latency становила приблизно `298.7 ms`, а replay
RAM estimate для `64` features - `24 KiB`. Перехід до `MicroFlow-32` зменшив
latency до приблизно `172.0 ms` і replay RAM estimate до `12 KiB`.

Цей trade-off є важливим саме для ESP32-class MCU. У desktop або сильнішому MCU
контексті `64` features могли б бути прийнятнішими, але для `ESP32-WROOM-32`
з `320 KB SRAM` зменшення feature dimension напряму знижує replay memory і
покращує latency. Саме тому `MicroFlow-32` використано як основний embedded path,
а `MicroFlow-64` лишено як reference.

Важливо, що `MicroFlow` не є центральною науковою новизною цієї роботи. Це
практичний frozen feature extractor backend. Основний внесок полягає у
resource-transparent CL architecture поверх frozen features.

### 4. Interpretation of FIFO vs reservoir

У поточному main pilot обидва replay modes змістили prediction distribution на
upstairs-like segment після supervised labels. `FIFO` досяг `88.57%`
accepted-rate для upstairs-like segment, а `reservoir-per-class` - `93.94%`.

Це дозволяє зробити обережний висновок:

```text
Both FIFO and reservoir-per-class replay can be implemented under the same
12 KiB replay budget and both produced prediction shifts after supervised labels.
```

Водночас поточний pilot занадто малий для твердження, що reservoir статистично
кращий за FIFO. Reservoir показав трохи сильніший результат у цьому pilot, але
для strict comparison потрібні повтори, більше subjects, довші sessions і
контрольованіші label timing. Тому reservoir у цій роботі краще описувати як
promising або slightly stronger in this pilot, а не як доведено superior.

FIFO залишається важливим baseline, а не помилкою. Його роль - показати, скільки
можна отримати від максимально простої replay policy за того самого memory
budget. Це робить reservoir comparison чеснішим.

### 5. What the real-device pilot demonstrates

Pilot `Sitting vs upstairs-like vertical hand-motion` демонструє, що pipeline
реагує на supervised labels на реальному IMU-сигналі. У `no_adapt` режимі
upstairs-like segment залишився класифікованим як `Sitting`, хоча confidence
падав під час руху. У `FIFO` і `reservoir` режимах після labels/train predictions
змістились до `Upstairs` або `Downstairs`.

Це важливий практичний сигнал: система не просто виконує synthetic update, а
змінює prediction behavior у фізичному pilot. Проте цей pilot не є повним HAR
benchmark. Він короткий, виконувався біля host PC, а upward motion був
обмежений USB-кабелем. Тому його треба описувати як real-device pilot sanity
check або real-device motion validation, а не як staircase benchmark.

Accepted labels для upstairs-like segment включали `Upstairs` і `Downstairs`,
бо vertical hand-motion може активувати обидва stair-like класи. Це не є
проблемою для pilot interpretation: ціль була перевірити prediction shift із
`Sitting` до stair-like classes, а не розв'язати повну upstairs/downstairs
дискримінацію.

### 6. Domain shift and why adaptation is needed

Окремий MPU6050 vs WISDM probe показав, що stationary MPU6050 distribution
відрізняється від WISDM normalization center. Особливо це видно по axis values,
де gravity і конкретна орієнтація сенсора створюють distribution shift.

Цей факт важливий для мотивації роботи. Offline WISDM model не гарантує
стабільної поведінки на конкретному реальному сенсорі, конкретному кріпленні і
конкретному користувачі. Навіть якщо offline model має прийнятну LOSO accuracy,
embedded deployment стикається з:

- іншим sensor hardware;
- іншою орієнтацією IMU;
- іншою амплітудою рухів;
- короткими constrained pilot sessions;
- user-level variation.

Тому continual adaptation on-device є практично вмотивованою, навіть якщо
поточна робота ще не виконує великий multi-subject real-device benchmark.

### 7. Label acquisition and UART limitations

У поточному setup labels надходять через USB/UART від оператора. Це є
structured supervised feedback channel. Такий protocol підходить для
контрольованих експериментів, але не є готовим deployment UX.

USB/UART має дві ролі:

- збір logs;
- передача supervised labels.

Це обмежує природність руху, бо плата фізично прив'язана до host PC. Проте це не
центральна проблема роботи і не має зміщувати фокус статті. Головний результат -
не в способі передачі labels, а в тому, що після отримання supervised labels
ESP32 виконує RAM-only replay update і змінює prediction behavior.

У future work labels можна передавати через:

- BLE;
- локальний UI;
- mobile companion app;
- scripted experiment controller;
- semi-autonomous feedback mechanism.

Але ці механізми не потрібні для доведення основного resource result.

### 8. Why persistence is future work

Persistence weights/replay state свідомо винесено з поточного scope. Це важливе
архітектурне рішення, а не недопрацювання.

Якщо додавати non-volatile CL state, треба розв'язувати окремі питання:

- формат storage для weights;
- формат storage для replay buffer;
- flash wear policy;
- update frequency;
- recovery після power loss;
- consistency між weights і replay samples;
- lifecycle management для довгих sessions.

Ці питання швидко перетворюють роботу на storage engineering або flash wear
study. Поточна стаття фокусується на RAM-only in-session continual learning:
чи може ESP32 виконувати replay-based update, скільки це коштує, і чи змінює це
prediction behavior у pilot. Persistence є природним Future Work, але не
обов'язковою частиною baseline study.

### 9. Limitations

Поточна робота має кілька обмежень:

1. **Pilot scale.** Real-device pilot короткий і не замінює full multi-subject
   `6-class HAR` evaluation.
2. **Motion constraints.** Рухи виконувались біля host PC, а USB cable обмежував
   природність movement.
3. **Label timing.** Labels надсилались burst-ами, тому частина labels
   прив'язана до latest feature vector, а не рівномірно до всього segment.
4. **Inference latency.** Synchronous `MicroFlow-32` inference близько `172 ms`
   не є strict `20 Hz` inference throughput.
5. **No persistence.** Weights і replay state не зберігаються після reset.
6. **No autonomous labeling.** Labels є supervised operator feedback, а не
   automatic activity labels.
7. **No statistical FIFO/reservoir conclusion.** Поточні pilot results
   демонструють feasibility і prediction shift, але не доводять статистичну
   перевагу reservoir.

Ці limitations мають бути явно описані, щоб не перебільшувати claims. Водночас
вони не скасовують основний внесок: resource-constrained RAM-only CL pipeline на
ESP32 працює end-to-end.

### 10. Future work

Найближчі напрями майбутньої роботи:

- повторити real-device pilots з кількома subjects;
- провести довші sessions для всіх `6` HAR classes;
- зробити рівномірніший label protocol замість burst labels;
- протестувати BLE або mobile-based supervised feedback;
- оптимізувати frozen feature extractor latency;
- інструментувати peak stack / high-water mark;
- додати non-volatile persistence з окремою flash wear policy;
- перевірити ESP32-S3 або плату з PSRAM для більших feature/replay layouts;
- порівняти більші replay budgets після стабілізації baseline.

Усі ці напрями мають розширювати baseline, але не змінюють поточний висновок:
мінімальний replay-based CL pipeline уже можна реалізувати на ESP32-class MCU з
прозорими resource metrics.

### 11. Main interpretation

Найкоротша інтерпретація результатів:

> The frozen feature extractor is the expensive part. The online continual
> learning update is cheap enough to run on ESP32. A small RAM-only replay buffer
> is sufficient to demonstrate supervised prediction shift in a real-device pilot.

Українською:

> Дорогою частиною є frozen feature extraction, а не online CL update. Малий
> RAM-only replay buffer достатній, щоб на ESP32 показати supervised prediction
> shift у pilot з реальним MPU6050.

Саме це і є центральним змістом роботи.

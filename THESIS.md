# THESIS.md

## Purpose

Цей файл фіксує робочу thesis-рамку для майбутньої статті, але не замінює `PLAN.md`.
Його призначення — зафіксувати вузький, узгоджений scope дослідження, щоб уникати scope drift під час реалізації.

## Core Thesis

Ми не намагаємося побудувати максимально складну continual learning систему для ESP32.
Натомість ми перевіряємо, чи можна **реалістично реалізувати мінімалістичний continual learning pipeline** на `ESP32-WROOM-32` у стеку `Rust/no_std` для задачі `IMU-HAR`, і чи дає `reservoir-per-class replay` кращий компроміс, ніж простий `FIFO replay`, за однакового memory budget.

## Main Research Question

Чи можливо розгорнути на `ESP32-WROOM-32` мінімальний `TinyOL`-подібний pipeline для `HAR`, де:

- ознаковий екстрактор натренований офлайн і залишається `frozen`
- на пристрої оновлюється лише легкий `OnlineLayer`
- адаптація виконується через `mini-batch SGD`
- catastrophic forgetting пом'якшується через replay buffer
- `FIFO` і `reservoir-per-class` порівнюються під однаковим RAM budget

## Canonical Framing

Найсильніше формулювання роботи:

> We show that a minimal continual learning pipeline for IMU-based human activity recognition can be realistically implemented on an `ESP32-WROOM-32` using a `Rust/no_std` firmware stack, and we compare `FIFO` and `reservoir-per-class` replay as two memory-constrained adaptation strategies under user-level domain shift.

Коротка англомовна версія:

> This work investigates whether a minimal continual learning pipeline can be deployed on an `ESP32-WROOM-32` microcontroller for IMU-based human activity recognition. A frozen feature extractor is trained offline, while a lightweight Rust-implemented online classification layer is updated on-device using mini-batch SGD. We compare `FIFO` and `per-class reservoir replay` under the same memory budget and evaluate their effect on personalization, forgetting, latency, RAM usage, flash footprint, and continual learning step time.

## Positioning Against Recent Work

Найближчий прямий сучасний аналог — `COOL (2026)`, де також розглядається `HAR + on-device continual learning + MCU`, але через складніший `KAN`-based classifier і `STM32H743`.

Наша робота **не** намагається конкурувати з такими підходами за максимальною `accuracy` або архітектурною складністю. Натомість ми свідомо фокусуємося на:

- `ESP32-WROOM-32`, а не потужнішому MCU-класі
- `Rust/no_std` firmware stack
- реальному `MPU6050`
- мінімальному replay-based CL baseline
- порівнянні `FIFO` vs `reservoir-per-class` за однакового memory budget
- явному вимірюванні `RAM`, `Flash`, `latency` і `CL step time`

Тобто сила роботи має бути не в твердженні `state of the art`, а у відтворюваному, ресурсно-прозорому embedded baseline між pure TinyML inference papers і складнішими HAR+CL системами.

## Experimental Framing

Експериментальний дизайн має бути простим і чистим:

- `No adaptation`
- `TinyOL + FIFO replay`
- `TinyOL + reservoir-per-class replay`

Основні метрики:

- `accuracy`
- `per-class accuracy`
- `forgetting`
- `latency`
- `RAM usage`
- `Flash footprint`
- `CL step time`

## What Counts As Novelty

Достатня і чесна новизна цієї роботи:

- перенесення `TinyOL`-style continual learning на `ESP32-WROOM-32`
- домен `IMU-HAR` з `WISDM + MPU6050`
- `Rust/no_std` реалізація `OnlineLayer` і `ReplayBuffer`
- порівняння `FIFO` vs `reservoir-per-class` при однаковому memory budget
- аналіз `domain shift` для нового користувача
- ресурсний профайл: `RAM`, `Flash`, `latency`, `CL step time`

## Must-Have Scope

Те, що входить у обов'язковий MVP роботи:

- `WISDM -> windowing 80x3 -> CNN -> feature vector 64` на PC
- embedded sensor path для `MPU6050` на `20 Hz`
- `frozen` inference / feature path на ESP32
- `OnlineLayer: 64 -> 6` у Rust
- `ReplayBuffer` у Rust
- реалізація `FIFO`
- реалізація `reservoir-per-class`
- supervised labels через `UART`
- вимірювання ресурсів і якості

## Out Of Scope For Now

Це не є ядром поточної роботи і не повинно роздувати MVP:

- `LWF`
- `CWR`
- `EWC`
- `iCaRL`
- `PQ` або інша latent compression
- `DMA` як обов'язкова частина внеску
- `RTC Fast RAM` як обов'язкова частина внеску
- `raw flash partitions` як обов'язкова частина внеску
- складний persistence layer
- autonomous label acquisition
- `microflow-rs` як обов'язковий inference runtime

## Clarifications

### FIFO is a baseline, not a mistake

`FIFO` не треба викидати. У цій роботі він виконує роль простого контрольного baseline, відносно якого вимірюється, чи справді `reservoir-per-class` дає кращу стійкість до forgetting за тих самих обмежень пам'яті.

### Labels are supervised via UART

У поточному experimental protocol мітки не добуваються автономно.
Під час експериментів `ground-truth` надходить від оператора через `UART` з PC.
Це означає, що робота оцінює **resource-constrained supervised adaptation**, а не fully autonomous continual learning.

### 320 KB SRAM is part of the point

Ми свідомо тримаємо таргет на `ESP32-WROOM-32` з `320 KB SRAM`.
Це робить роботу практично цінною: ми не спираємося на `PSRAM`, `ESP32-S3` або сильніші edge-платформи.

## Role Of This File

`THESIS.md` потрібен для фіксації дослідницької рамки до початку повної реалізації.
Під час розробки він має використовуватись як короткий анти-scope-drift reference.
Детальний технічний план лишається в `PLAN.md`.

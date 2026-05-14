#![no_std]
#![no_main]

// Phase 4 UART + replay smoke test.
// Verifies UART labels -> synthetic features -> ReplayBuffer32 -> OnlineLayer32 update.
// No sensor, no MicroFlow, no main.rs integration, no persistence.

use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    main,
    time::{Duration, Instant},
    uart::{Config as UartConfig, Uart},
};
use esp32_cl_har::{
    model::{CLASS_LABELS, MICROFLOW32_FEATURE_TENSOR_SIZE},
    online_layer::OnlineLayer32,
    replay_buffer::{ReplayBuffer32, ReplayStrategy},
};
use log::{info, warn};

#[cfg(feature = "replay_fifo_policy")]
const ACTIVE_POLICY: ReplayStrategy = ReplayStrategy::Fifo;
#[cfg(not(feature = "replay_fifo_policy"))]
const ACTIVE_POLICY: ReplayStrategy = ReplayStrategy::Reservoir;
const BATCH_SIZE: usize = 12;
const IDLE_DELAY: Duration = Duration::from_millis(10);
const LABELS_PER_UPDATE: u32 = 10;
const LEARNING_RATE: f32 = 0.001;
const TICK_INTERVAL: Duration = Duration::from_millis(1_000);

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

fn init() -> esp_hal::peripherals::Peripherals {
    esp_println::logger::init_logger(log::LevelFilter::Info);
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    esp_hal::init(config)
}

fn busy_wait(duration: Duration) {
    let started = Instant::now();
    while started.elapsed() < duration {}
}

fn parse_label(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'5' => Some(byte - b'0'),
        b'\r' | b'\n' | b' ' | b'\t' => None,
        _ => None,
    }
}

fn policy_name(policy: ReplayStrategy) -> &'static str {
    match policy {
        ReplayStrategy::Fifo => "fifo",
        ReplayStrategy::Reservoir => "reservoir",
    }
}

fn synthetic_features(label: u8, sample_idx: u32) -> [f32; MICROFLOW32_FEATURE_TENSOR_SIZE] {
    let mut features = [0.0; MICROFLOW32_FEATURE_TENSOR_SIZE];
    let base = label as f32 * 0.25 + sample_idx as f32 * 0.01;
    let mut i = 0;
    while i < features.len() {
        features[i] = base + i as f32 * 0.001;
        i += 1;
    }
    features
}

#[main]
fn main() -> ! {
    let peripherals = init();
    let mut led = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());
    let mut uart = Uart::new(peripherals.UART0, UartConfig::default())
        .expect("UART0 init failed")
        .with_rx(peripherals.GPIO3);

    let mut replay = ReplayBuffer32::with_seed(0x1234_5678);
    let mut layer = OnlineLayer32::new_microflow32_pretrained();
    let mut batch_features = [[0.0; MICROFLOW32_FEATURE_TENSOR_SIZE]; BATCH_SIZE];
    let mut batch_labels = [0_u8; BATCH_SIZE];

    let mut buf = [0_u8; 8];
    let mut invalid_seen = 0_u32;
    let mut labels_seen = 0_u32;
    let mut labels_since_update = 0_u32;
    let mut train_steps = 0_u32;
    let mut ticks = 0_u32;
    let mut led_on = false;
    let mut last_tick = Instant::now();

    info!("uart replay smoke started");
    info!(
        "policy={}, labels_per_update={}, batch_size={}, lr={}",
        policy_name(ACTIVE_POLICY),
        LABELS_PER_UPDATE,
        BATCH_SIZE,
        LEARNING_RATE,
    );
    info!("send one-character labels over UART0/USB serial: 0..5");
    info!("no sensor, no MicroFlow, no main.rs integration, no flash writes");

    loop {
        match uart.read_buffered(&mut buf) {
            Ok(0) => {}
            Ok(count) => {
                for &byte in &buf[..count] {
                    let Some(label) = parse_label(byte) else {
                        if !matches!(byte, b'\r' | b'\n' | b' ' | b'\t') {
                            invalid_seen = invalid_seen.saturating_add(1);
                            warn!("LABEL_INVALID byte={} total_invalid={}", byte, invalid_seen);
                        }
                        continue;
                    };

                    labels_seen = labels_seen.saturating_add(1);
                    labels_since_update = labels_since_update.saturating_add(1);
                    let features = synthetic_features(label, labels_seen);

                    let push_started = Instant::now();
                    let insert = replay.push(label, features, ACTIVE_POLICY);
                    let push_us = push_started.elapsed().as_micros();

                    let added = insert.is_some();
                    let class_len = replay.class_len(label).unwrap_or(0);
                    info!(
                        "LABEL label={} name={} added={} class_len={} buffer_len={} push_us={} total_seen={}",
                        label,
                        CLASS_LABELS[usize::from(label)],
                        added as u8,
                        class_len,
                        replay.total_len(),
                        push_us,
                        replay.total_seen(),
                    );

                    if labels_since_update >= LABELS_PER_UPDATE {
                        let sample_started = Instant::now();
                        let batch_len =
                            replay.sample_balanced_batch(&mut batch_features, &mut batch_labels);
                        let sample_us = sample_started.elapsed().as_micros();

                        let update_started = Instant::now();
                        layer.backward_batch(
                            &batch_features[..batch_len],
                            &batch_labels[..batch_len],
                            LEARNING_RATE,
                        );
                        let update_us = update_started.elapsed().as_micros();

                        train_steps = train_steps.saturating_add(1);
                        labels_since_update = 0;
                        info!(
                            "TRAIN policy={} step={} batch_len={} sample_us={} update_us={} total_seen={} buffer_len={}",
                            policy_name(ACTIVE_POLICY),
                            train_steps,
                            batch_len,
                            sample_us,
                            update_us,
                            replay.total_seen(),
                            replay.total_len(),
                        );
                    }
                }
            }
            Err(_) => {
                invalid_seen = invalid_seen.saturating_add(1);
                warn!("LABEL_RX_ERROR total_invalid={}", invalid_seen);
            }
        }

        if last_tick.elapsed() >= TICK_INTERVAL {
            ticks = ticks.saturating_add(1);
            info!(
                "UART_REPLAY_SMOKE tick={} labels={} train_steps={} buffer_len={} invalid={}",
                ticks,
                labels_seen,
                train_steps,
                replay.total_len(),
                invalid_seen,
            );

            led_on = !led_on;
            if led_on {
                led.set_high();
            } else {
                led.set_low();
            }
            last_tick = Instant::now();
        }

        busy_wait(IDLE_DELAY);
    }
}

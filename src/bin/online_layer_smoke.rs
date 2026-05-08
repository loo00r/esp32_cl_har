#![no_std]
#![no_main]

// Archived Phase 4a checkpoint.
// Tests OnlineLayer math on ESP32 using synthetic features only.
// No frozen backend, no sensor loop, no replay, no persistence.

use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    main,
    time::{Duration, Instant},
};
use esp32_cl_har::{
    model::{CLASS_LABELS, FEATURE_TENSOR_SIZE},
    online_layer::OnlineLayer64,
};
use log::info;

const IDLE_DELAY: Duration = Duration::from_millis(1_000);
const SMOKE_LR: f32 = 0.01;

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

fn synthetic_features(seed: f32) -> [f32; FEATURE_TENSOR_SIZE] {
    let mut features = [0.0; FEATURE_TENSOR_SIZE];
    let mut i = 0;
    while i < FEATURE_TENSOR_SIZE {
        let x = i as f32;
        features[i] = seed + (x * 0.03125) - 1.0;
        i += 1;
    }
    features
}

fn argmax(values: &[f32]) -> usize {
    let mut best_idx = 0;
    let mut best_value = values[0];
    let mut i = 1;
    while i < values.len() {
        if values[i] > best_value {
            best_value = values[i];
            best_idx = i;
        }
        i += 1;
    }
    best_idx
}

#[main]
fn main() -> ! {
    let peripherals = init();
    let mut led = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());

    let mut layer = OnlineLayer64::new();
    let sample_a = synthetic_features(0.25);
    let sample_b = synthetic_features(-0.10);
    let batch = [sample_a, sample_b];
    let labels = [0_u8, 0_u8];
    let mut led_on = false;

    info!("online layer smoke test started");
    info!("no sensor, no frozen backend, no flash writes");

    loop {
        let forward_started = Instant::now();
        let before = layer.forward(&sample_a);
        let forward_us = forward_started.elapsed().as_micros();

        let backward_started = Instant::now();
        layer.backward_batch(&batch, &labels, SMOKE_LR);
        let backward_us = backward_started.elapsed().as_micros();

        let after = layer.forward(&sample_a);
        let predicted_idx = argmax(&after);

        info!(
            "forward_us={}, backward_us={}, before_p0={}, after_p0={}, pred={}({})",
            forward_us,
            backward_us,
            before[0],
            after[0],
            predicted_idx,
            CLASS_LABELS[predicted_idx],
        );

        led_on = !led_on;
        if led_on {
            led.set_high();
        } else {
            led.set_low();
        }
        busy_wait(IDLE_DELAY);
    }
}

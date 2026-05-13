#![no_std]
#![no_main]

// Diagnostic-only smoke binary.
// It exercises MicroFlow's public f32 API, where MicroFlow quantizes input
// internally. Keep it for comparison; do not use it as the main ESP32 pipeline.

use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    main,
    time::{Duration, Instant},
};
use esp32_cl_har::{
    inference_microflow::MicroflowFeatureBackend,
    model::{FEATURE_COUNT, FEATURE_TENSOR_SIZE, WINDOW_SIZE},
};
use log::info;

const IDLE_DELAY: Duration = Duration::from_millis(1_000);

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

fn synthetic_window() -> [f32; WINDOW_SIZE * FEATURE_COUNT] {
    let mut input = [0.0; WINDOW_SIZE * FEATURE_COUNT];
    let mut i = 0;
    while i < input.len() {
        input[i] = (i as f32 * 0.0125) - 1.0;
        i += 1;
    }
    input
}

fn checksum(values: &[f32; FEATURE_TENSOR_SIZE]) -> f32 {
    let mut sum = 0.0;
    let mut i = 0;
    while i < values.len() {
        sum += values[i];
        i += 1;
    }
    sum
}

#[main]
fn main() -> ! {
    let peripherals = init();
    let mut led = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());
    let mut led_on = false;

    let backend = MicroflowFeatureBackend::new();
    let input = synthetic_window();

    let started = Instant::now();
    let features = backend.extract_features(&input);
    let elapsed_us = started.elapsed().as_micros();
    let checksum = checksum(&features);

    info!("microflow feature smoke prepared");
    info!("backend={}", backend.backend_name());
    info!(
        "latency_us={}, checksum={}, f0={}, f1={}, f2={}, f3={}",
        elapsed_us, checksum, features[0], features[1], features[2], features[3],
    );

    loop {
        led_on = !led_on;
        if led_on {
            led.set_high();
        } else {
            led.set_low();
        }
        busy_wait(IDLE_DELAY);
    }
}

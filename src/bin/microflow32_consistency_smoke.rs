#![no_std]
#![no_main]

// PC-vs-ESP consistency smoke for MicroFlow-32.
// Uses the same deterministic quantized input as the Python TFLite check.

use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    main,
    time::{Duration, Instant},
};
use esp32_cl_har::{
    inference_microflow32::Microflow32FeatureBackend,
    model::{FEATURE_COUNT, INPUT_TENSOR_SIZE, MICROFLOW32_FEATURE_TENSOR_SIZE, WINDOW_SIZE},
    quant::{INPUT_SCALE, INPUT_ZERO_POINT, quantize_scalar},
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

fn consistency_input() -> [i8; INPUT_TENSOR_SIZE] {
    let mut input = [0; INPUT_TENSOR_SIZE];
    let mut i = 0;
    while i < input.len() {
        let normalized = (i as f32 * 0.0125) - 1.0;
        input[i] = quantize_scalar(normalized, INPUT_SCALE, INPUT_ZERO_POINT);
        i += 1;
    }
    input
}

fn checksum(values: &[f32; MICROFLOW32_FEATURE_TENSOR_SIZE]) -> f32 {
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

    let backend = Microflow32FeatureBackend::new();
    let input = consistency_input();

    let started = Instant::now();
    let features = backend.extract_features_quantized(&input);
    let elapsed_us = started.elapsed().as_micros();
    let checksum = checksum(&features);

    info!("microflow32 consistency smoke prepared");
    info!("backend={}", backend.backend_name());
    info!(
        "input_shape=[1,{},{},1], input_dtype=i8, output_shape=[1,1,1,{}]",
        WINDOW_SIZE, FEATURE_COUNT, MICROFLOW32_FEATURE_TENSOR_SIZE,
    );
    info!(
        "latency_us={}, checksum={}, f0={}, f1={}, f2={}, f3={}, f4={}, f5={}, f6={}, f7={}",
        elapsed_us,
        checksum,
        features[0],
        features[1],
        features[2],
        features[3],
        features[4],
        features[5],
        features[6],
        features[7],
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

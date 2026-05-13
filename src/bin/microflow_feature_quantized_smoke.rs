#![no_std]
#![no_main]

// Active Phase 3 smoke binary for the intended ESP32 inference boundary:
// i8[240] input tensor -> MicroFlow predict_quantized() -> f32[64] features.
// No sensor loop, no CL, no replay, no persistence.

use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    main,
    time::{Duration, Instant},
};
use esp32_cl_har::{
    inference_microflow::MicroflowFeatureBackend,
    model::{FEATURE_COUNT, FEATURE_TENSOR_SIZE, INPUT_TENSOR_SIZE, WINDOW_SIZE},
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

fn synthetic_quantized_window() -> [i8; INPUT_TENSOR_SIZE] {
    let mut input = [0; INPUT_TENSOR_SIZE];
    let mut i = 0;
    while i < input.len() {
        let normalized = (i as f32 * 0.0125) - 1.0;
        input[i] = quantize_scalar(normalized, INPUT_SCALE, INPUT_ZERO_POINT);
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
    let input = synthetic_quantized_window();

    let started = Instant::now();
    let features = backend.extract_features_quantized(&input);
    let elapsed_us = started.elapsed().as_micros();
    let checksum = checksum(&features);

    info!("microflow quantized feature smoke prepared");
    info!("backend={}", backend.backend_name());
    info!(
        "input_shape=[1,{},{},1], input_dtype=i8, output_shape=[1,1,1,{}]",
        WINDOW_SIZE, FEATURE_COUNT, FEATURE_TENSOR_SIZE,
    );
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

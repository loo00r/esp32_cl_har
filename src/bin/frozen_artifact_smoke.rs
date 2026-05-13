#![no_std]
#![no_main]

// Archived Phase 3 checkpoint.
// Verifies that frozen model artifacts are embedded as read-only bytes.
// Not part of the active inference path.

use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    main,
    time::{Duration, Instant},
};
use esp32_cl_har::model::{
    BASELINE_CLASSIFIER_ARTIFACT, BASELINE_CLASSIFIER_MODEL_BYTES, CLASSIFIER_OUTPUT_SCALE,
    CLASSIFIER_OUTPUT_SHAPE, CLASSIFIER_OUTPUT_ZERO_POINT, FEATURE_EXTRACTOR_ARTIFACT,
    FEATURE_EXTRACTOR_MODEL_BYTES, FEATURE_OUTPUT_SCALE, FEATURE_OUTPUT_SHAPE,
    FEATURE_OUTPUT_ZERO_POINT, MODEL_INPUT_SCALE, MODEL_INPUT_SHAPE, MODEL_INPUT_ZERO_POINT,
    simple_checksum32,
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

#[main]
fn main() -> ! {
    let peripherals = init();
    let mut led = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());
    let mut led_on = false;

    let classifier_checksum = simple_checksum32(BASELINE_CLASSIFIER_MODEL_BYTES);
    let feature_checksum = simple_checksum32(FEATURE_EXTRACTOR_MODEL_BYTES);

    info!("frozen artifact smoke test started");
    info!(
        "input_shape={:?}, input_scale={}, input_zero_point={}",
        MODEL_INPUT_SHAPE, MODEL_INPUT_SCALE, MODEL_INPUT_ZERO_POINT
    );
    info!(
        "classifier: name={}, bytes={}, checksum=0x{:08X}, output_shape={:?}, output_scale={}, output_zero_point={}",
        BASELINE_CLASSIFIER_ARTIFACT,
        BASELINE_CLASSIFIER_MODEL_BYTES.len(),
        classifier_checksum,
        CLASSIFIER_OUTPUT_SHAPE,
        CLASSIFIER_OUTPUT_SCALE,
        CLASSIFIER_OUTPUT_ZERO_POINT,
    );
    info!(
        "feature_extractor: name={}, bytes={}, checksum=0x{:08X}, output_shape={:?}, output_scale={}, output_zero_point={}",
        FEATURE_EXTRACTOR_ARTIFACT,
        FEATURE_EXTRACTOR_MODEL_BYTES.len(),
        feature_checksum,
        FEATURE_OUTPUT_SHAPE,
        FEATURE_OUTPUT_SCALE,
        FEATURE_OUTPUT_ZERO_POINT,
    );
    info!("artifacts embedded as read-only firmware assets");

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

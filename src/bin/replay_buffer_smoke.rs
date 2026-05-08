#![no_std]
#![no_main]

// Phase 4 replay buffer smoke test.
// Verifies fixed-size per-class reservoir/FIFO storage without sensor, inference,
// UART labels, persistence, heap allocation, or flash writes.

use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    main,
    time::{Duration, Instant},
};
use esp32_cl_har::{
    model::{MICROFLOW32_FEATURE_TENSOR_SIZE, NUM_CLASSES},
    online_layer::OnlineLayer32,
    replay_buffer::{ReplayBuffer32, ReplayStrategy},
};
use log::info;

const IDLE_DELAY: Duration = Duration::from_millis(1_000);
const BATCH_SIZE: usize = 12;

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

fn synthetic_features(label: u8, sample_idx: usize) -> [f32; MICROFLOW32_FEATURE_TENSOR_SIZE] {
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
    let mut led_on = false;

    let mut replay = ReplayBuffer32::with_seed(0x1234_5678);

    let started = Instant::now();
    for class_idx in 0..NUM_CLASSES {
        for sample_idx in 0..24 {
            let label = class_idx as u8;
            let features = synthetic_features(label, sample_idx);
            replay.push(label, features, ReplayStrategy::Reservoir);
        }
    }
    let fill_us = started.elapsed().as_micros();

    let mut batch_features = [[0.0; MICROFLOW32_FEATURE_TENSOR_SIZE]; BATCH_SIZE];
    let mut batch_labels = [0_u8; BATCH_SIZE];
    let sample_started = Instant::now();
    let batch_len = replay.sample_balanced_batch(&mut batch_features, &mut batch_labels);
    let sample_us = sample_started.elapsed().as_micros();

    let mut layer = OnlineLayer32::new_microflow32_pretrained();
    let before = layer.forward(&batch_features[0])[usize::from(batch_labels[0])];
    let update_started = Instant::now();
    layer.backward_batch(
        &batch_features[..batch_len],
        &batch_labels[..batch_len],
        0.001,
    );
    let update_us = update_started.elapsed().as_micros();
    let after = layer.forward(&batch_features[0])[usize::from(batch_labels[0])];

    info!("replay buffer smoke prepared");
    info!(
        "classes={}, slots_per_class={}, feature_dim={}, total_seen={}, total_len={}",
        NUM_CLASSES,
        esp32_cl_har::replay_buffer::REPLAY_SLOTS_PER_CLASS,
        MICROFLOW32_FEATURE_TENSOR_SIZE,
        replay.total_seen(),
        replay.total_len(),
    );
    info!(
        "timing_us: fill={}, sample={}, online_update={}",
        fill_us, sample_us, update_us,
    );
    info!(
        "batch_len={}, first_label={}, target_prob_before={}, target_prob_after={}",
        batch_len, batch_labels[0], before, after,
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

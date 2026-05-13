#![no_std]
#![no_main]

use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    main,
    time::{Duration, Instant},
};
use esp32_cl_har::{
    inference_microflow32::Microflow32FeatureBackend,
    model::{CLASS_LABELS, INPUT_TENSOR_SIZE, NUM_CLASSES},
    online_layer::OnlineLayer32,
};
use log::info;

const TAG: &str = "balanced_600";
const WINDOWS: &[u8] = include_bytes!("../eval_artifacts/wisdm_eval_windows_i8_balanced_600.bin");
const LABELS: &[u8] = include_bytes!("../eval_artifacts/wisdm_eval_labels_u8_balanced_600.bin");
const PROGRESS_EVERY: usize = 20;
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

fn argmax(values: &[f32; NUM_CLASSES]) -> usize {
    let mut best_idx = 0;
    let mut best_value = values[0];
    let mut idx = 1;
    while idx < NUM_CLASSES {
        if values[idx] > best_value {
            best_idx = idx;
            best_value = values[idx];
        }
        idx += 1;
    }
    best_idx
}

#[main]
fn main() -> ! {
    let peripherals = init();
    let mut led = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());

    let backend = Microflow32FeatureBackend::new();
    let head = OnlineLayer32::new_microflow32_pretrained();
    let total = LABELS.len();

    info!("WISDM_EVAL_START tag={} total={}", TAG, total);

    if WINDOWS.len() != total * INPUT_TENSOR_SIZE {
        info!(
            "WISDM_EVAL_ERROR tag={} windows_bytes={} labels_bytes={} expected_windows_bytes={}",
            TAG,
            WINDOWS.len(),
            LABELS.len(),
            total * INPUT_TENSOR_SIZE,
        );
        loop {
            led.toggle();
            busy_wait(IDLE_DELAY);
        }
    }

    let mut confusion = [[0_u32; NUM_CLASSES]; NUM_CLASSES];
    let mut correct = 0_u32;
    let mut min_infer_us = u64::MAX;
    let mut max_infer_us = 0_u64;
    let mut total_infer_us = 0_u64;

    let mut idx = 0;
    while idx < total {
        let mut input = [0_i8; INPUT_TENSOR_SIZE];
        let offset = idx * INPUT_TENSOR_SIZE;
        let mut value_idx = 0;
        while value_idx < INPUT_TENSOR_SIZE {
            input[value_idx] = WINDOWS[offset + value_idx] as i8;
            value_idx += 1;
        }

        let started = Instant::now();
        let features = backend.extract_features_quantized(&input);
        let infer_us = started.elapsed().as_micros();
        let probs = head.forward(&features);
        let pred = argmax(&probs);
        let truth = LABELS[idx] as usize;

        if infer_us < min_infer_us {
            min_infer_us = infer_us;
        }
        if infer_us > max_infer_us {
            max_infer_us = infer_us;
        }
        total_infer_us += infer_us;

        if truth < NUM_CLASSES {
            confusion[truth][pred] += 1;
            if pred == truth {
                correct += 1;
            }
        }

        let done = idx + 1;
        if done % PROGRESS_EVERY == 0 || done == total {
            let acc = correct as f32 / done as f32;
            info!(
                "WISDM_EVAL_PROGRESS idx={} total={} correct={} acc={}",
                done, total, correct, acc,
            );
        }

        idx += 1;
    }

    let mean_infer_us = if total > 0 {
        total_infer_us / total as u64
    } else {
        0
    };
    let accuracy = if total > 0 {
        correct as f32 / total as f32
    } else {
        0.0
    };

    info!(
        "WISDM_EVAL_SUMMARY tag={} total={} correct={} accuracy={} mean_infer_us={} min_infer_us={} max_infer_us={}",
        TAG, total, correct, accuracy, mean_infer_us, min_infer_us, max_infer_us,
    );

    let mut class_idx = 0;
    while class_idx < NUM_CLASSES {
        let mut support = 0_u32;
        let mut pred_idx = 0;
        while pred_idx < NUM_CLASSES {
            support += confusion[class_idx][pred_idx];
            pred_idx += 1;
        }
        let class_correct = confusion[class_idx][class_idx];
        let recall = if support > 0 {
            class_correct as f32 / support as f32
        } else {
            0.0
        };
        info!(
            "WISDM_EVAL_CLASS class={} name={} support={} correct={} recall={}",
            class_idx, CLASS_LABELS[class_idx], support, class_correct, recall,
        );
        class_idx += 1;
    }

    let mut true_idx = 0;
    while true_idx < NUM_CLASSES {
        info!(
            "WISDM_EVAL_CONF true={} pred0={} pred1={} pred2={} pred3={} pred4={} pred5={}",
            true_idx,
            confusion[true_idx][0],
            confusion[true_idx][1],
            confusion[true_idx][2],
            confusion[true_idx][3],
            confusion[true_idx][4],
            confusion[true_idx][5],
        );
        true_idx += 1;
    }

    loop {
        led.toggle();
        busy_wait(IDLE_DELAY);
    }
}

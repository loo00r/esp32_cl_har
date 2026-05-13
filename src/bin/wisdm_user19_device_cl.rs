#![no_std]
#![no_main]

// Isolated target-user WISDM CL evaluation.
// No sensor path, no UART labels, no persistence, no main.rs integration.

use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    main,
    time::{Duration, Instant},
};
use esp32_cl_har::{
    model::{CLASS_LABELS, FEATURE_COUNT, INPUT_TENSOR_SIZE, MICROFLOW32_FEATURE_TENSOR_SIZE, NUM_CLASSES, WINDOW_SIZE},
    online_layer::OnlineLayer32,
    replay_buffer::{ReplayBuffer32, ReplayStrategy},
};
use log::info;
use microflow::{buffer::Buffer2D, buffer::Buffer4D, model};

type MicroflowQuantizedInput = Buffer4D<i8, 1, WINDOW_SIZE, FEATURE_COUNT, 1>;
type MicroflowOutput = Buffer4D<f32, 1, 1, 1, MICROFLOW32_FEATURE_TENSOR_SIZE>;

#[model("results/fold_artifacts/wisdm_user19_microflow32/microflow32_user19_feature_extractor_int8.tflite")]
pub struct WisdmUser19FeatureExtractor;

include!("../eval_artifacts/wisdm_user19_head.rs");

const TAG: &str = "wisdm_user19_target_cl";
const TARGET_USER: u8 = 19;
const WINDOWS: &[u8] =
    include_bytes!("../../results/fold_artifacts/wisdm_user19_microflow32/wisdm_user19_target_windows_i8.bin");
const LABELS: &[u8] =
    include_bytes!("../../results/fold_artifacts/wisdm_user19_microflow32/wisdm_user19_target_labels_u8.bin");

const BATCH_SIZE: usize = 12;
const BUDGET_PER_CLASS: usize = 10;
const LABELS_PER_UPDATE: u32 = 10;
const LEARNING_RATE: f32 = 0.01;
const IDLE_DELAY: Duration = Duration::from_millis(1_000);

#[cfg(feature = "replay_fifo_policy")]
const ACTIVE_POLICY: ReplayStrategy = ReplayStrategy::Fifo;
#[cfg(not(feature = "replay_fifo_policy"))]
const ACTIVE_POLICY: ReplayStrategy = ReplayStrategy::Reservoir;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

#[derive(Clone, Copy)]
struct EvalStats {
    total: u32,
    correct: u32,
    confusion: [[u32; NUM_CLASSES]; NUM_CLASSES],
    infer_total_us: u64,
    infer_min_us: u64,
    infer_max_us: u64,
}

impl EvalStats {
    const fn new() -> Self {
        Self {
            total: 0,
            correct: 0,
            confusion: [[0; NUM_CLASSES]; NUM_CLASSES],
            infer_total_us: 0,
            infer_min_us: u64::MAX,
            infer_max_us: 0,
        }
    }
}

fn init() -> esp_hal::peripherals::Peripherals {
    esp_println::logger::init_logger(log::LevelFilter::Info);
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    esp_hal::init(config)
}

fn busy_wait(duration: Duration) {
    let started = Instant::now();
    while started.elapsed() < duration {}
}

fn policy_name(policy: ReplayStrategy) -> &'static str {
    match policy {
        ReplayStrategy::Fifo => "fifo",
        ReplayStrategy::Reservoir => "reservoir",
    }
}

fn make_quantized_input(input: &[i8; INPUT_TENSOR_SIZE]) -> MicroflowQuantizedInput {
    [Buffer2D::from_fn(|row, col| {
        [input[row * FEATURE_COUNT + col]]
    })]
}

fn extract_features(input: &[i8; INPUT_TENSOR_SIZE]) -> [f32; MICROFLOW32_FEATURE_TENSOR_SIZE] {
    let input = make_quantized_input(input);
    let output: MicroflowOutput = WisdmUser19FeatureExtractor::predict_quantized(input);
    output[0][(0, 0)]
}

fn load_window(idx: usize) -> [i8; INPUT_TENSOR_SIZE] {
    let mut input = [0_i8; INPUT_TENSOR_SIZE];
    let offset = idx * INPUT_TENSOR_SIZE;
    let mut value_idx = 0;
    while value_idx < INPUT_TENSOR_SIZE {
        input[value_idx] = WINDOWS[offset + value_idx] as i8;
        value_idx += 1;
    }
    input
}

fn new_user19_head() -> OnlineLayer32 {
    OnlineLayer32 {
        weights: WISDM_USER_HEAD_WEIGHTS,
        bias: WISDM_USER_HEAD_BIAS,
    }
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

fn selected_for_adaptation(idx: usize, label: u8, selected_seen: &[usize; NUM_CLASSES]) -> bool {
    let class_idx = label as usize;
    if class_idx >= NUM_CLASSES {
        return false;
    }

    // PC gate uses the first N windows per class, but always leaves at least
    // one held-out sample per class when support is non-zero.
    selected_seen[class_idx] < BUDGET_PER_CLASS && selected_seen[class_idx] + 1 < class_support(label, idx)
}

fn class_support(label: u8, upto_idx: usize) -> usize {
    let mut total = 0;
    let mut idx = 0;
    while idx < LABELS.len() {
        if LABELS[idx] == label {
            total += 1;
        }
        idx += 1;
    }
    let _ = upto_idx;
    total
}

fn build_adaptation_mask(mask: &mut [bool]) -> usize {
    let mut selected_seen = [0_usize; NUM_CLASSES];
    let mut selected_total = 0;
    let mut idx = 0;
    while idx < LABELS.len() {
        let label = LABELS[idx];
        if selected_for_adaptation(idx, label, &selected_seen) {
            let class_idx = label as usize;
            selected_seen[class_idx] += 1;
            mask[idx] = true;
            selected_total += 1;
        } else {
            mask[idx] = false;
        }
        idx += 1;
    }
    selected_total
}

fn evaluate(
    phase: &str,
    split: &str,
    layer: &OnlineLayer32,
    adaptation_mask: &[bool],
    use_held_out_only: bool,
) -> EvalStats {
    let mut stats = EvalStats::new();
    let mut idx = 0;
    while idx < LABELS.len() {
        if use_held_out_only && adaptation_mask[idx] {
            idx += 1;
            continue;
        }

        let input = load_window(idx);
        let started = Instant::now();
        let features = extract_features(&input);
        let infer_us = started.elapsed().as_micros();
        let probs = layer.forward(&features);
        let pred = argmax(&probs);
        let truth = LABELS[idx] as usize;

        stats.total += 1;
        stats.infer_total_us += infer_us;
        if infer_us < stats.infer_min_us {
            stats.infer_min_us = infer_us;
        }
        if infer_us > stats.infer_max_us {
            stats.infer_max_us = infer_us;
        }

        if truth < NUM_CLASSES {
            stats.confusion[truth][pred] += 1;
            if pred == truth {
                stats.correct += 1;
            }
        }

        idx += 1;
    }

    print_eval(phase, split, &stats);
    stats
}

fn print_eval(phase: &str, split: &str, stats: &EvalStats) {
    let accuracy = if stats.total > 0 {
        stats.correct as f32 / stats.total as f32
    } else {
        0.0
    };
    let mean_infer_us = if stats.total > 0 {
        stats.infer_total_us / stats.total as u64
    } else {
        0
    };
    info!(
        "WISDM_CL_EVAL phase={} split={} total={} correct={} accuracy={} mean_infer_us={} min_infer_us={} max_infer_us={}",
        phase,
        split,
        stats.total,
        stats.correct,
        accuracy,
        mean_infer_us,
        stats.infer_min_us,
        stats.infer_max_us,
    );

    let mut class_idx = 0;
    while class_idx < NUM_CLASSES {
        let mut support = 0_u32;
        let mut pred_idx = 0;
        while pred_idx < NUM_CLASSES {
            support += stats.confusion[class_idx][pred_idx];
            pred_idx += 1;
        }
        let class_correct = stats.confusion[class_idx][class_idx];
        let recall = if support > 0 {
            class_correct as f32 / support as f32
        } else {
            0.0
        };
        info!(
            "WISDM_CL_CLASS phase={} split={} class={} name={} support={} correct={} recall={}",
            phase,
            split,
            class_idx,
            CLASS_LABELS[class_idx],
            support,
            class_correct,
            recall,
        );
        class_idx += 1;
    }
}

fn adapt(layer: &mut OnlineLayer32, adaptation_mask: &[bool]) -> u32 {
    let mut replay = ReplayBuffer32::with_seed(0x1234_5678);
    let mut batch_features = [[0.0; MICROFLOW32_FEATURE_TENSOR_SIZE]; BATCH_SIZE];
    let mut batch_labels = [0_u8; BATCH_SIZE];
    let mut labels_since_update = 0_u32;
    let mut train_steps = 0_u32;
    let mut adapted_labels = 0_u32;

    let mut idx = 0;
    while idx < LABELS.len() {
        if !adaptation_mask[idx] {
            idx += 1;
            continue;
        }

        let input = load_window(idx);
        let started = Instant::now();
        let features = extract_features(&input);
        let infer_us = started.elapsed().as_micros();
        let label = LABELS[idx];
        replay.push(label, features, ACTIVE_POLICY);
        adapted_labels += 1;
        labels_since_update += 1;

        info!(
            "WISDM_CL_LABEL idx={} label={} name={} buffer_len={} total_seen={} infer_us={}",
            idx,
            label,
            CLASS_LABELS[label as usize],
            replay.total_len(),
            replay.total_seen(),
            infer_us,
        );

        if labels_since_update >= LABELS_PER_UPDATE {
            let sample_started = Instant::now();
            let batch_len = replay.sample_balanced_batch(&mut batch_features, &mut batch_labels);
            let sample_us = sample_started.elapsed().as_micros();
            let update_started = Instant::now();
            layer.backward_batch(
                &batch_features[..batch_len],
                &batch_labels[..batch_len],
                LEARNING_RATE,
            );
            let update_us = update_started.elapsed().as_micros();
            train_steps += 1;
            labels_since_update = 0;
            info!(
                "WISDM_CL_TRAIN policy={} step={} batch_len={} sample_us={} update_us={} adapted_labels={} buffer_len={} total_seen={}",
                policy_name(ACTIVE_POLICY),
                train_steps,
                batch_len,
                sample_us,
                update_us,
                adapted_labels,
                replay.total_len(),
                replay.total_seen(),
            );
        }

        idx += 1;
    }

    info!(
        "WISDM_CL_ADAPT_DONE policy={} adapted_labels={} train_steps={} buffer_len={} total_seen={}",
        policy_name(ACTIVE_POLICY),
        adapted_labels,
        train_steps,
        replay.total_len(),
        replay.total_seen(),
    );
    train_steps
}

#[main]
fn main() -> ! {
    let peripherals = init();
    let mut led = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());
    let total = LABELS.len();
    let mut adaptation_mask = [false; 208];

    info!(
        "WISDM_CL_START tag={} target_user={} total={} budget_per_class={} lr={} labels_per_update={} batch_size={} policy={}",
        TAG,
        TARGET_USER,
        total,
        BUDGET_PER_CLASS,
        LEARNING_RATE,
        LABELS_PER_UPDATE,
        BATCH_SIZE,
        policy_name(ACTIVE_POLICY),
    );

    if WINDOWS.len() != total * INPUT_TENSOR_SIZE || total != adaptation_mask.len() {
        info!(
            "WISDM_CL_ERROR windows_bytes={} labels_bytes={} expected_windows_bytes={} mask_len={}",
            WINDOWS.len(),
            LABELS.len(),
            total * INPUT_TENSOR_SIZE,
            adaptation_mask.len(),
        );
        loop {
            led.toggle();
            busy_wait(IDLE_DELAY);
        }
    }

    let selected_total = build_adaptation_mask(&mut adaptation_mask);
    info!(
        "WISDM_CL_SPLIT selected_adaptation={} held_out={} total={}",
        selected_total,
        total - selected_total,
        total,
    );

    let mut layer = new_user19_head();
    evaluate("pre", "all", &layer, &adaptation_mask, false);
    evaluate("pre", "held_out", &layer, &adaptation_mask, true);
    let train_steps = adapt(&mut layer, &adaptation_mask);
    evaluate("post", "held_out", &layer, &adaptation_mask, true);
    info!(
        "WISDM_CL_SUMMARY tag={} target_user={} policy={} budget_per_class={} lr={} adapted_labels={} train_steps={}",
        TAG,
        TARGET_USER,
        policy_name(ACTIVE_POLICY),
        BUDGET_PER_CLASS,
        LEARNING_RATE,
        selected_total,
        train_steps,
    );

    loop {
        led.toggle();
        busy_wait(IDLE_DELAY);
    }
}

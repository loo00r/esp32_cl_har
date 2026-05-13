use crate::{
    model::{FEATURE_COUNT, FEATURE_TENSOR_SIZE, INPUT_TENSOR_SIZE, WINDOW_SIZE},
    window::SlidingWindow,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalizationStats {
    pub means: [f32; FEATURE_COUNT],
    pub stds: [f32; FEATURE_COUNT],
}

pub const WISDM_ZSCORE_STATS: NormalizationStats = NormalizationStats {
    means: [0.664_113, 7.246_045, 0.397_697],
    stds: [6.876_277, 6.739_789, 4.761_111],
};

// Default MPU6050 accel full-scale is ±2g after reset, which corresponds to 16384 LSB/g.
pub const MPU6050_LSB_PER_G: f32 = 16_384.0;
pub const STANDARD_GRAVITY_MPS2: f32 = 9.806_65;

pub const INPUT_SCALE: f32 = 0.030_599_216;
pub const INPUT_ZERO_POINT: i8 = 9;

pub const CLASSIFIER_OUTPUT_SCALE: f32 = 0.003_906_25;
pub const CLASSIFIER_OUTPUT_ZERO_POINT: i8 = -128;

pub const FEATURE_OUTPUT_SCALE: f32 = 0.050_572_72;
pub const FEATURE_OUTPUT_ZERO_POINT: i8 = -128;

pub fn quantize_window(window: &SlidingWindow, out: &mut [i8; INPUT_TENSOR_SIZE]) {
    debug_assert!(window.is_full());

    for sample_index in 0..WINDOW_SIZE {
        let sample = window.ordered_sample(sample_index);
        let base = sample_index * FEATURE_COUNT;
        for axis in 0..FEATURE_COUNT {
            let normalized = normalize_axis(sample[axis], axis);
            out[base + axis] = quantize_scalar(normalized, INPUT_SCALE, INPUT_ZERO_POINT);
        }
    }
}

pub fn dequantize_feature_tensor(
    quantized: &[i8; FEATURE_TENSOR_SIZE],
    out: &mut [f32; FEATURE_TENSOR_SIZE],
) {
    for i in 0..FEATURE_TENSOR_SIZE {
        out[i] = dequantize_scalar(
            quantized[i],
            FEATURE_OUTPUT_SCALE,
            FEATURE_OUTPUT_ZERO_POINT,
        );
    }
}

fn normalize_axis(raw: i16, axis: usize) -> f32 {
    let value = raw_accel_to_mps2(raw);
    (value - WISDM_ZSCORE_STATS.means[axis]) / WISDM_ZSCORE_STATS.stds[axis]
}

pub fn raw_accel_to_mps2(raw: i16) -> f32 {
    (raw as f32 / MPU6050_LSB_PER_G) * STANDARD_GRAVITY_MPS2
}

pub fn quantize_scalar(value: f32, scale: f32, zero_point: i8) -> i8 {
    let scaled = value / scale;
    let rounded = if scaled >= 0.0 {
        (scaled + 0.5) as i32
    } else {
        (scaled - 0.5) as i32
    };
    let centered = rounded + zero_point as i32;
    centered.clamp(i8::MIN as i32, i8::MAX as i32) as i8
}

pub fn dequantize_scalar(value: i8, scale: f32, zero_point: i8) -> f32 {
    (i32::from(value) - i32::from(zero_point)) as f32 * scale
}

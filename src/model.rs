pub const SAMPLE_RATE_HZ: u32 = 20;

pub const WINDOW_SIZE: usize = 80;
pub const WINDOW_STRIDE: usize = 40;
pub const FEATURE_COUNT: usize = 3;
pub const INPUT_TENSOR_SIZE: usize = WINDOW_SIZE * FEATURE_COUNT;

pub const NUM_CLASSES: usize = 6;
pub const FEATURE_TENSOR_SIZE: usize = 64;
pub const MODEL_INPUT_SHAPE: [usize; 4] = [1, WINDOW_SIZE, FEATURE_COUNT, 1];
pub const CLASSIFIER_OUTPUT_SHAPE: [usize; 4] = [1, 1, 1, NUM_CLASSES];
pub const FEATURE_OUTPUT_SHAPE: [usize; 4] = [1, 1, 1, FEATURE_TENSOR_SIZE];

pub const CLASS_LABELS: [&str; NUM_CLASSES] = [
    "Walking",
    "Jogging",
    "Upstairs",
    "Downstairs",
    "Sitting",
    "Standing",
];

pub const BASELINE_CLASSIFIER_ARTIFACT: &str = "microflow_fullconv_classifier_int8.tflite";
pub const FEATURE_EXTRACTOR_ARTIFACT: &str = "microflow_fullconv_feature_extractor_int8.tflite";

pub const MODEL_INPUT_SCALE: f32 = 0.030_599_216;
pub const MODEL_INPUT_ZERO_POINT: i32 = 9;
pub const CLASSIFIER_OUTPUT_SCALE: f32 = 0.003_906_25;
pub const CLASSIFIER_OUTPUT_ZERO_POINT: i32 = -128;
pub const FEATURE_OUTPUT_SCALE: f32 = 0.050_572_72;
pub const FEATURE_OUTPUT_ZERO_POINT: i32 = -128;

pub static BASELINE_CLASSIFIER_MODEL_BYTES: &[u8] =
    include_bytes!("model_artifacts/microflow_fullconv_classifier_int8.tflite");
pub static FEATURE_EXTRACTOR_MODEL_BYTES: &[u8] =
    include_bytes!("model_artifacts/microflow_fullconv_feature_extractor_int8.tflite");

pub fn simple_checksum32(bytes: &[u8]) -> u32 {
    let mut hash = 0x811c_9dc5_u32;
    for &byte in bytes {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

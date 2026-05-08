pub const SAMPLE_RATE_HZ: u32 = 20;

pub const WINDOW_SIZE: usize = 80;
pub const WINDOW_STRIDE: usize = 40;
pub const FEATURE_COUNT: usize = 3;
pub const INPUT_TENSOR_SIZE: usize = WINDOW_SIZE * FEATURE_COUNT;

pub const NUM_CLASSES: usize = 6;
pub const FEATURE_TENSOR_SIZE: usize = 64;

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

use crate::model::{FEATURE_COUNT, FEATURE_TENSOR_SIZE, WINDOW_SIZE};
use microflow::buffer::Buffer2D;
use microflow::buffer::Buffer4D;
use microflow::model;

// MicroFlow candidate backend for the frozen feature extractor only.
// This is not the CL contribution; OnlineLayer/replay stay in separate Rust modules.
type MicroflowInput = Buffer4D<f32, 1, WINDOW_SIZE, FEATURE_COUNT, 1>;
type MicroflowQuantizedInput = Buffer4D<i8, 1, WINDOW_SIZE, FEATURE_COUNT, 1>;
type MicroflowOutput = Buffer4D<f32, 1, 1, 1, FEATURE_TENSOR_SIZE>;

#[model("src/model_artifacts/microflow_fullconv_feature_extractor_int8.tflite")]
pub struct MicroflowFeatureExtractor;

pub struct MicroflowFeatureBackend;

impl MicroflowFeatureBackend {
    pub const fn new() -> Self {
        Self
    }

    pub fn backend_name(&self) -> &'static str {
        "microflow-fullconv-feature-extractor"
    }

    pub fn make_input(input: &[f32; WINDOW_SIZE * FEATURE_COUNT]) -> MicroflowInput {
        [Buffer2D::from_fn(|row, col| {
            [input[row * FEATURE_COUNT + col]]
        })]
    }

    pub fn make_quantized_input(
        input: &[i8; WINDOW_SIZE * FEATURE_COUNT],
    ) -> MicroflowQuantizedInput {
        [Buffer2D::from_fn(|row, col| {
            [input[row * FEATURE_COUNT + col]]
        })]
    }

    pub fn extract_features(
        &self,
        input: &[f32; WINDOW_SIZE * FEATURE_COUNT],
    ) -> [f32; FEATURE_TENSOR_SIZE] {
        // Diagnostic path only. MicroFlow quantizes this f32 input internally.
        // The ESP32 production-like path should use extract_features_quantized().
        let input = Self::make_input(input);
        let output: MicroflowOutput = MicroflowFeatureExtractor::predict(input);
        output[0][(0, 0)]
    }

    pub fn extract_features_quantized(
        &self,
        input: &[i8; WINDOW_SIZE * FEATURE_COUNT],
    ) -> [f32; FEATURE_TENSOR_SIZE] {
        // Intended Phase 3 path:
        // normalized window -> i8[240] -> INT8 MicroFlow graph -> f32[64] features.
        let input = Self::make_quantized_input(input);
        let output: MicroflowOutput = MicroflowFeatureExtractor::predict_quantized(input);
        output[0][(0, 0)]
    }
}

use crate::model::{FEATURE_COUNT, MICROFLOW32_FEATURE_TENSOR_SIZE, WINDOW_SIZE};
use microflow::buffer::Buffer2D;
use microflow::buffer::Buffer4D;
use microflow::model;

// MicroFlow-32 ablation backend for the frozen feature extractor only.
// It mirrors inference_microflow.rs but returns 32 latent features.
type MicroflowQuantizedInput = Buffer4D<i8, 1, WINDOW_SIZE, FEATURE_COUNT, 1>;
type MicroflowOutput = Buffer4D<f32, 1, 1, 1, MICROFLOW32_FEATURE_TENSOR_SIZE>;

#[model("src/model_artifacts/microflow_fullconv32_feature_extractor_int8.tflite")]
pub struct Microflow32FeatureExtractor;

pub struct Microflow32FeatureBackend;

impl Microflow32FeatureBackend {
    pub const fn new() -> Self {
        Self
    }

    pub fn backend_name(&self) -> &'static str {
        "microflow-fullconv32-feature-extractor"
    }

    pub fn make_quantized_input(
        input: &[i8; WINDOW_SIZE * FEATURE_COUNT],
    ) -> MicroflowQuantizedInput {
        [Buffer2D::from_fn(|row, col| [input[row * FEATURE_COUNT + col]])]
    }

    pub fn extract_features_quantized(
        &self,
        input: &[i8; WINDOW_SIZE * FEATURE_COUNT],
    ) -> [f32; MICROFLOW32_FEATURE_TENSOR_SIZE] {
        let input = Self::make_quantized_input(input);
        let output: MicroflowOutput = Microflow32FeatureExtractor::predict_quantized(input);
        output[0][(0, 0)]
    }
}

use crate::model::{FEATURE_COUNT, FEATURE_TENSOR_SIZE, WINDOW_SIZE};
use microflow::buffer::Buffer2D;
use microflow::buffer::Buffer4D;
use microflow::model;

type MicroflowInput = Buffer4D<f32, 1, WINDOW_SIZE, FEATURE_COUNT, 1>;
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
        [Buffer2D::from_fn(|row, col| [input[row * FEATURE_COUNT + col]])]
    }

    pub fn extract_features(
        &self,
        input: &[f32; WINDOW_SIZE * FEATURE_COUNT],
    ) -> [f32; FEATURE_TENSOR_SIZE] {
        let input = Self::make_input(input);
        let output: MicroflowOutput = MicroflowFeatureExtractor::predict(input);
        output[0][(0, 0)]
    }
}

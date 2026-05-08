use crate::model::{FEATURE_TENSOR_SIZE, INPUT_TENSOR_SIZE, NUM_CLASSES};

// Placeholder boundary for the main sensor loop.
// The active MicroFlow feasibility work lives behind the `microflow_backend`
// feature in `inference_microflow.rs`; this stub keeps default firmware builds
// simple until the real frozen backend is wired into the loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InferenceError {
    BackendUnavailable,
}

pub struct FrozenInferenceBackend;

impl FrozenInferenceBackend {
    pub const fn new() -> Self {
        Self
    }

    pub const fn backend_name(&self) -> &'static str {
        "tflm-backend-stub"
    }

    pub fn classify(
        &self,
        input: &[i8; INPUT_TENSOR_SIZE],
        output: &mut [i8; NUM_CLASSES],
    ) -> Result<(), InferenceError> {
        let _ = input;
        *output = [0; NUM_CLASSES];
        Err(InferenceError::BackendUnavailable)
    }

    pub fn extract_features(
        &self,
        input: &[i8; INPUT_TENSOR_SIZE],
        output: &mut [i8; FEATURE_TENSOR_SIZE],
    ) -> Result<(), InferenceError> {
        let _ = input;
        *output = [0; FEATURE_TENSOR_SIZE];
        Err(InferenceError::BackendUnavailable)
    }
}

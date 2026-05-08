use crate::model::{FEATURE_TENSOR_SIZE, NUM_CLASSES};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OnlineLayer {
    pub weights: [[f32; NUM_CLASSES]; FEATURE_TENSOR_SIZE],
    pub bias: [f32; NUM_CLASSES],
}

impl OnlineLayer {
    pub const fn new() -> Self {
        Self {
            weights: [[0.0; NUM_CLASSES]; FEATURE_TENSOR_SIZE],
            bias: [0.0; NUM_CLASSES],
        }
    }

    pub fn forward_logits(&self, features: &[f32; FEATURE_TENSOR_SIZE]) -> [f32; NUM_CLASSES] {
        let mut logits = self.bias;
        for (feature_idx, &feature_value) in features.iter().enumerate() {
            for (class_idx, logit) in logits.iter_mut().enumerate() {
                *logit += feature_value * self.weights[feature_idx][class_idx];
            }
        }
        logits
    }

    pub fn forward(&self, features: &[f32; FEATURE_TENSOR_SIZE]) -> [f32; NUM_CLASSES] {
        softmax(self.forward_logits(features))
    }

    pub fn backward_batch(
        &mut self,
        batch: &[[f32; FEATURE_TENSOR_SIZE]],
        labels: &[u8],
        lr: f32,
    ) {
        if batch.is_empty() || batch.len() != labels.len() || lr <= 0.0 {
            return;
        }

        let inv_batch = 1.0 / batch.len() as f32;
        let mut grad_w = [[0.0; NUM_CLASSES]; FEATURE_TENSOR_SIZE];
        let mut grad_b = [0.0; NUM_CLASSES];

        for (features, &label) in batch.iter().zip(labels.iter()) {
            let probs = self.forward(features);
            let mut delta = probs;
            let label_idx = usize::from(label);
            if label_idx < NUM_CLASSES {
                delta[label_idx] -= 1.0;
            }

            for class_idx in 0..NUM_CLASSES {
                grad_b[class_idx] += delta[class_idx];
            }

            for feature_idx in 0..FEATURE_TENSOR_SIZE {
                let feature_value = features[feature_idx];
                for class_idx in 0..NUM_CLASSES {
                    grad_w[feature_idx][class_idx] += feature_value * delta[class_idx];
                }
            }
        }

        for feature_idx in 0..FEATURE_TENSOR_SIZE {
            for class_idx in 0..NUM_CLASSES {
                self.weights[feature_idx][class_idx] -= lr * grad_w[feature_idx][class_idx] * inv_batch;
            }
        }

        for class_idx in 0..NUM_CLASSES {
            self.bias[class_idx] -= lr * grad_b[class_idx] * inv_batch;
        }
    }
}

pub fn softmax(logits: [f32; NUM_CLASSES]) -> [f32; NUM_CLASSES] {
    let mut max_logit = logits[0];
    for &value in logits.iter().skip(1) {
        if value > max_logit {
            max_logit = value;
        }
    }

    let mut exp_values = [0.0; NUM_CLASSES];
    let mut sum = 0.0;
    for (idx, &value) in logits.iter().enumerate() {
        let exp_value = exp_approx(value - max_logit);
        exp_values[idx] = exp_value;
        sum += exp_value;
    }

    let mut probs = [0.0; NUM_CLASSES];
    if sum > 0.0 {
        for idx in 0..NUM_CLASSES {
            probs[idx] = exp_values[idx] / sum;
        }
    }
    probs
}

fn exp_approx(x: f32) -> f32 {
    let mut term = 1.0;
    let mut sum = 1.0;
    for n in 1..=8 {
        term *= x / n as f32;
        sum += term;
    }
    if sum > 0.0 { sum } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_hot_like_features(scale: f32) -> [f32; FEATURE_TENSOR_SIZE] {
        let mut features = [0.0; FEATURE_TENSOR_SIZE];
        features[0] = scale;
        features[1] = scale * 0.5;
        features[2] = scale * 0.25;
        features
    }

    #[test]
    fn forward_outputs_probabilities() {
        let layer = OnlineLayer::new();
        let probs = layer.forward(&[0.0; FEATURE_TENSOR_SIZE]);
        let total: f32 = probs.iter().sum();
        assert!((total - 1.0).abs() < 1e-4);
        for p in probs {
            assert!(p >= 0.0);
        }
    }

    #[test]
    fn backward_batch_updates_weights() {
        let mut layer = OnlineLayer::new();
        let batch = [one_hot_like_features(1.0), one_hot_like_features(0.8)];
        let labels = [0_u8, 0_u8];

        let before = layer.weights[0][0];
        layer.backward_batch(&batch, &labels, 0.1);
        let after = layer.weights[0][0];

        assert_ne!(before, after);
    }

    #[test]
    fn repeated_training_improves_target_class_probability() {
        let mut layer = OnlineLayer::new();
        let sample = one_hot_like_features(1.0);
        let before = layer.forward(&sample)[0];

        let batch = [sample];
        let labels = [0_u8];
        for _ in 0..25 {
            layer.backward_batch(&batch, &labels, 0.2);
        }

        let after = layer.forward(&sample)[0];
        assert!(after > before);
    }
}

use crate::model::{FEATURE_TENSOR_SIZE, MICROFLOW32_FEATURE_TENSOR_SIZE, NUM_CLASSES};
use libm::expf;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OnlineLayer<const D: usize> {
    pub weights: [[f32; NUM_CLASSES]; D],
    pub bias: [f32; NUM_CLASSES],
}

pub type OnlineLayer64 = OnlineLayer<FEATURE_TENSOR_SIZE>;
pub type OnlineLayer32 = OnlineLayer<MICROFLOW32_FEATURE_TENSOR_SIZE>;

// Dequantized equivalent of the INT8 `microflow_fullconv32_classifier` 1x1 Conv2D head.
// Layout is [feature][class], matching OnlineLayer forward.
pub const MICROFLOW32_HEAD_WEIGHTS: [[f32; NUM_CLASSES]; MICROFLOW32_FEATURE_TENSOR_SIZE] = [
    [
        0.446479917,
        0.373507887,
        -0.514188647,
        0.165268317,
        -0.148890689,
        -0.645077229,
    ],
    [
        -0.151116282,
        0.158022568,
        0.41530624,
        -0.106243916,
        -0.638102949,
        -0.140234187,
    ],
    [
        0.0618202947,
        -0.308862299,
        -0.145027578,
        -0.0295121986,
        0.900434136,
        0.588983595,
    ],
    [
        -0.707498908,
        -0.725467265,
        0.237317845,
        -0.177073196,
        0.311961442,
        0.448749393,
    ],
    [
        -0.872353077,
        -0.222668171,
        0.0131843248,
        0.247902468,
        0.375771731,
        -0.299166262,
    ],
    [
        -0.0755581409,
        0.380690724,
        0.065921627,
        0.407268345,
        0.0141800651,
        -0.962941408,
    ],
    [
        -0.109902747,
        -0.466884851,
        0.646031916,
        -0.200682953,
        0.453762084,
        -0.336562037,
    ],
    [
        -0.0206067655,
        0.380690724,
        -0.0395529754,
        -0.0472195186,
        -0.21979101,
        -1.18731606,
    ],
    [
        0.384659618,
        -0.567444682,
        0.237317845,
        0.00590243982,
        -0.432491988,
        0.205676809,
    ],
    [
        0.130509511,
        0.136474043,
        -0.388937593,
        -0.424975663,
        -0.560112596,
        -0.102838404,
    ],
    [
        -0.377790689,
        0.660821676,
        -0.0329608135,
        -0.200682953,
        -0.120530553,
        -0.86945194,
    ],
    [
        0.59072727,
        -0.373507887,
        0.125251085,
        -0.578439116,
        0.0567202605,
        0.23372364,
    ],
    [
        0.233543336,
        -0.65363878,
        0.309831619,
        0.424975663,
        0.233971074,
        0.0560936742,
    ],
    [
        0.370921761,
        0.165205419,
        0.197764874,
        -0.129853681,
        -0.517572403,
        0.439400434,
    ],
    [
        0.473955601,
        -0.524347603,
        0.0725137889,
        -0.45448786,
        0.26942125,
        -0.0560936742,
    ],
    [
        -0.700630009,
        -0.402239263,
        0.290055156,
        0.424975663,
        -0.0425401963,
        -0.0560936742,
    ],
    [
        -0.501431286,
        -0.308862299,
        0.0856981128,
        0.306926876,
        0.354501635,
        0.0280468371,
    ],
    [
        0.405266374,
        0.567444682,
        -0.336200297,
        0.165268317,
        -0.361591667,
        -0.766613543,
    ],
    [
        0.0961649045,
        -0.201119632,
        -0.0988824368,
        0.383658588,
        -0.241061106,
        -0.373957813,
    ],
    [
        0.261019021,
        0.165205419,
        -0.0922902748,
        0.13575612,
        -0.382861763,
        -0.130885243,
    ],
    [
        0.329708248,
        -0.0933769718,
        -0.0856981128,
        -0.749609828,
        0.326141506,
        -0.794660389,
    ],
    [
        -0.281625777,
        -0.574627519,
        0.18458055,
        0.413170785,
        0.155980721,
        0.47679624,
    ],
    [
        0.549513757,
        -0.373507887,
        -0.41530624,
        -0.430878103,
        -0.538842499,
        0.0841405094,
    ],
    [
        0.597596169,
        -0.258582383,
        0.145027578,
        -0.401365906,
        -0.248151138,
        -0.00934894569,
    ],
    [
        0.487693429,
        -0.316045135,
        -0.118658923,
        -0.566634238,
        0.241061106,
        -0.299166262,
    ],
    [
        0.199198723,
        -0.452519178,
        -0.309831619,
        -0.584341526,
        0.0850803927,
        0.55158782,
    ],
    [
        0.219805494,
        -0.574627519,
        0.151619732,
        0.218390271,
        -0.545932531,
        -0.261770487,
    ],
    [
        -0.494562358,
        0.373507887,
        0.408714056,
        0.242000028,
        0.18434085,
        -0.654426217,
    ],
    [
        -0.769319236,
        -0.689553022,
        0.257094324,
        0.153463438,
        0.560112596,
        0.532889903,
    ],
    [
        0.103033826,
        -0.912221193,
        0.237317845,
        0.118048795,
        0.198520914,
        0.560936749,
    ],
    [
        0.680023253,
        0.416604936,
        -0.797651649,
        -0.0177073199,
        0.0,
        -0.149583131,
    ],
    [
        0.0824270621,
        -0.301679432,
        0.837204635,
        -0.118048795,
        0.0567202605,
        0.271119416,
    ],
];

pub const MICROFLOW32_HEAD_BIAS: [f32; NUM_CLASSES] = [
    -0.00804937817,
    -0.0473470315,
    0.0333143063,
    0.0190212056,
    -0.055563014,
    0.0972310007,
];

impl<const D: usize> OnlineLayer<D> {
    pub const fn new() -> Self {
        Self {
            weights: [[0.0; NUM_CLASSES]; D],
            bias: [0.0; NUM_CLASSES],
        }
    }

    pub fn forward_logits(&self, features: &[f32; D]) -> [f32; NUM_CLASSES] {
        let mut logits = self.bias;
        for (feature_idx, &feature_value) in features.iter().enumerate() {
            for (class_idx, logit) in logits.iter_mut().enumerate() {
                *logit += feature_value * self.weights[feature_idx][class_idx];
            }
        }
        logits
    }

    pub fn forward(&self, features: &[f32; D]) -> [f32; NUM_CLASSES] {
        softmax(self.forward_logits(features))
    }

    pub fn backward_batch(&mut self, batch: &[[f32; D]], labels: &[u8], lr: f32) {
        if batch.is_empty() || batch.len() != labels.len() || lr <= 0.0 {
            return;
        }

        let inv_batch = 1.0 / batch.len() as f32;
        let mut grad_w = [[0.0; NUM_CLASSES]; D];
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

            for feature_idx in 0..D {
                let feature_value = features[feature_idx];
                for class_idx in 0..NUM_CLASSES {
                    grad_w[feature_idx][class_idx] += feature_value * delta[class_idx];
                }
            }
        }

        for feature_idx in 0..D {
            for class_idx in 0..NUM_CLASSES {
                self.weights[feature_idx][class_idx] -=
                    lr * grad_w[feature_idx][class_idx] * inv_batch;
            }
        }

        for class_idx in 0..NUM_CLASSES {
            self.bias[class_idx] -= lr * grad_b[class_idx] * inv_batch;
        }
    }
}

impl OnlineLayer<MICROFLOW32_FEATURE_TENSOR_SIZE> {
    pub const fn new_microflow32_pretrained() -> Self {
        Self {
            weights: MICROFLOW32_HEAD_WEIGHTS,
            bias: MICROFLOW32_HEAD_BIAS,
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
        let shifted = value - max_logit;
        let exp_value = expf(shifted);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn one_hot_like_features<const D: usize>(scale: f32) -> [f32; D] {
        let mut features = [0.0; D];
        features[0] = scale;
        features[1] = scale * 0.5;
        features[2] = scale * 0.25;
        features
    }

    #[test]
    fn forward_outputs_probabilities() {
        let layer = OnlineLayer64::new();
        let probs = layer.forward(&[0.0; FEATURE_TENSOR_SIZE]);
        let total: f32 = probs.iter().sum();
        assert!((total - 1.0).abs() < 1e-4);
        for p in probs {
            assert!(p >= 0.0);
        }
    }

    #[test]
    fn backward_batch_updates_weights() {
        let mut layer = OnlineLayer64::new();
        let batch = [
            one_hot_like_features::<FEATURE_TENSOR_SIZE>(1.0),
            one_hot_like_features::<FEATURE_TENSOR_SIZE>(0.8),
        ];
        let labels = [0_u8, 0_u8];

        let before = layer.weights[0][0];
        layer.backward_batch(&batch, &labels, 0.1);
        let after = layer.weights[0][0];

        assert_ne!(before, after);
    }

    #[test]
    fn repeated_training_improves_target_class_probability() {
        let mut layer = OnlineLayer64::new();
        let sample = one_hot_like_features::<FEATURE_TENSOR_SIZE>(1.0);
        let before = layer.forward(&sample)[0];

        let batch = [sample];
        let labels = [0_u8];
        for _ in 0..25 {
            layer.backward_batch(&batch, &labels, 0.2);
        }

        let after = layer.forward(&sample)[0];
        assert!(after > before);
    }

    #[test]
    fn online_layer32_forward_outputs_probabilities() {
        let layer = OnlineLayer32::new();
        let probs = layer.forward(&[0.0; MICROFLOW32_FEATURE_TENSOR_SIZE]);
        let total: f32 = probs.iter().sum();
        assert!((total - 1.0).abs() < 1e-4);
    }
}

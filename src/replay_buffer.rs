use crate::model::NUM_CLASSES;

pub const REPLAY_SLOTS_PER_CLASS: usize = 16;
pub const REPLAY_SEED: u32 = 0xC0DE_0420;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayStrategy {
    Fifo,
    Reservoir,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayInsert {
    pub class_idx: usize,
    pub slot_idx: usize,
    pub replaced: bool,
}

#[derive(Debug, PartialEq)]
pub struct ReplayBuffer<const D: usize> {
    pub features: [[[f32; D]; REPLAY_SLOTS_PER_CLASS]; NUM_CLASSES],
    pub seen: [u32; NUM_CLASSES],
    pub len: [u8; NUM_CLASSES],
    fifo_next: [u8; NUM_CLASSES],
    rng_state: u32,
}

impl<const D: usize> ReplayBuffer<D> {
    pub const fn new() -> Self {
        Self {
            features: [[[0.0; D]; REPLAY_SLOTS_PER_CLASS]; NUM_CLASSES],
            seen: [0; NUM_CLASSES],
            len: [0; NUM_CLASSES],
            fifo_next: [0; NUM_CLASSES],
            rng_state: REPLAY_SEED,
        }
    }

    pub const fn with_seed(seed: u32) -> Self {
        Self {
            features: [[[0.0; D]; REPLAY_SLOTS_PER_CLASS]; NUM_CLASSES],
            seen: [0; NUM_CLASSES],
            len: [0; NUM_CLASSES],
            fifo_next: [0; NUM_CLASSES],
            rng_state: seed,
        }
    }

    pub fn push(
        &mut self,
        label: u8,
        features: [f32; D],
        strategy: ReplayStrategy,
    ) -> Option<ReplayInsert> {
        match strategy {
            ReplayStrategy::Fifo => self.push_fifo(label, features),
            ReplayStrategy::Reservoir => self.push_reservoir(label, features),
        }
    }

    pub fn push_fifo(&mut self, label: u8, features: [f32; D]) -> Option<ReplayInsert> {
        let class_idx = Self::label_to_class(label)?;
        self.seen[class_idx] = self.seen[class_idx].saturating_add(1);

        if usize::from(self.len[class_idx]) < REPLAY_SLOTS_PER_CLASS {
            let slot_idx = usize::from(self.len[class_idx]);
            self.features[class_idx][slot_idx] = features;
            self.len[class_idx] += 1;
            self.fifo_next[class_idx] = self.len[class_idx] % REPLAY_SLOTS_PER_CLASS as u8;

            return Some(ReplayInsert {
                class_idx,
                slot_idx,
                replaced: false,
            });
        }

        let slot_idx = usize::from(self.fifo_next[class_idx]);
        self.features[class_idx][slot_idx] = features;
        self.fifo_next[class_idx] = ((slot_idx + 1) % REPLAY_SLOTS_PER_CLASS) as u8;

        Some(ReplayInsert {
            class_idx,
            slot_idx,
            replaced: true,
        })
    }

    pub fn push_reservoir(&mut self, label: u8, features: [f32; D]) -> Option<ReplayInsert> {
        let class_idx = Self::label_to_class(label)?;
        let seen_after = self.seen[class_idx].saturating_add(1);
        self.seen[class_idx] = seen_after;

        if usize::from(self.len[class_idx]) < REPLAY_SLOTS_PER_CLASS {
            let slot_idx = usize::from(self.len[class_idx]);
            self.features[class_idx][slot_idx] = features;
            self.len[class_idx] += 1;

            return Some(ReplayInsert {
                class_idx,
                slot_idx,
                replaced: false,
            });
        }

        let candidate = self.next_bounded(seen_after);
        if candidate < REPLAY_SLOTS_PER_CLASS as u32 {
            let slot_idx = candidate as usize;
            self.features[class_idx][slot_idx] = features;
            Some(ReplayInsert {
                class_idx,
                slot_idx,
                replaced: true,
            })
        } else {
            None
        }
    }

    pub fn sample_balanced_batch(
        &mut self,
        out_features: &mut [[f32; D]],
        out_labels: &mut [u8],
    ) -> usize {
        if out_features.is_empty() || out_features.len() != out_labels.len() {
            return 0;
        }

        let mut written = 0;
        let mut empty_classes_seen = 0;
        let mut class_idx = self.next_bounded(NUM_CLASSES as u32) as usize;

        while written < out_features.len() && empty_classes_seen < NUM_CLASSES {
            let class_len = usize::from(self.len[class_idx]);
            if class_len == 0 {
                empty_classes_seen += 1;
            } else {
                empty_classes_seen = 0;
                let slot_idx = self.next_bounded(class_len as u32) as usize;
                out_features[written] = self.features[class_idx][slot_idx];
                out_labels[written] = class_idx as u8;
                written += 1;
            }

            class_idx = (class_idx + 1) % NUM_CLASSES;
        }

        written
    }

    pub fn total_len(&self) -> usize {
        let mut total = 0;
        for &class_len in &self.len {
            total += usize::from(class_len);
        }
        total
    }

    pub fn total_seen(&self) -> u32 {
        let mut total = 0_u32;
        for &class_seen in &self.seen {
            total = total.saturating_add(class_seen);
        }
        total
    }

    pub fn class_len(&self, label: u8) -> Option<usize> {
        let class_idx = Self::label_to_class(label)?;
        Some(usize::from(self.len[class_idx]))
    }

    pub fn class_seen(&self, label: u8) -> Option<u32> {
        let class_idx = Self::label_to_class(label)?;
        Some(self.seen[class_idx])
    }

    pub fn slot(&self, label: u8, slot_idx: usize) -> Option<[f32; D]> {
        let class_idx = Self::label_to_class(label)?;
        if slot_idx >= usize::from(self.len[class_idx]) {
            return None;
        }
        Some(self.features[class_idx][slot_idx])
    }

    const fn label_to_class(label: u8) -> Option<usize> {
        if (label as usize) < NUM_CLASSES {
            Some(label as usize)
        } else {
            None
        }
    }

    fn next_bounded(&mut self, upper: u32) -> u32 {
        if upper <= 1 {
            return 0;
        }
        self.next_u32() % upper
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.rng_state;
        if x == 0 {
            x = REPLAY_SEED;
        }
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng_state = x;
        x
    }
}

pub type ReplayBuffer32 = ReplayBuffer<{ crate::model::MICROFLOW32_FEATURE_TENSOR_SIZE }>;
pub type ReplayBuffer64 = ReplayBuffer<{ crate::model::FEATURE_TENSOR_SIZE }>;

#[cfg(test)]
mod tests {
    use super::*;

    fn sample<const D: usize>(value: f32) -> [f32; D] {
        [value; D]
    }

    #[test]
    fn reservoir_keeps_per_class_capacity() {
        let mut buffer = ReplayBuffer::<4>::with_seed(1);

        for i in 0..64 {
            buffer.push_reservoir(2, sample(i as f32));
        }

        assert_eq!(buffer.class_seen(2), Some(64));
        assert_eq!(buffer.class_len(2), Some(REPLAY_SLOTS_PER_CLASS));
        assert_eq!(buffer.total_len(), REPLAY_SLOTS_PER_CLASS);
    }

    #[test]
    fn fifo_replaces_oldest_slot_per_class() {
        let mut buffer = ReplayBuffer::<2>::new();

        for i in 0..(REPLAY_SLOTS_PER_CLASS + 1) {
            buffer.push_fifo(0, sample(i as f32));
        }

        assert_eq!(buffer.class_len(0), Some(REPLAY_SLOTS_PER_CLASS));
        assert_eq!(
            buffer.slot(0, 0),
            Some(sample(REPLAY_SLOTS_PER_CLASS as f32))
        );
    }

    #[test]
    fn balanced_batch_samples_available_classes() {
        let mut buffer = ReplayBuffer::<2>::with_seed(2);
        buffer.push_reservoir(0, [1.0, 1.0]);
        buffer.push_reservoir(3, [3.0, 3.0]);

        let mut batch = [[0.0; 2]; 4];
        let mut labels = [0_u8; 4];
        let count = buffer.sample_balanced_batch(&mut batch, &mut labels);

        assert_eq!(count, 4);
        assert!(labels.iter().any(|&label| label == 0));
        assert!(labels.iter().any(|&label| label == 3));
    }
}

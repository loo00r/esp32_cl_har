use crate::model::{FEATURE_COUNT, WINDOW_SIZE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlidingWindow {
    samples: [[i16; FEATURE_COUNT]; WINDOW_SIZE],
    len: usize,
    write_index: usize,
}

impl SlidingWindow {
    pub const fn new() -> Self {
        Self {
            samples: [[0; FEATURE_COUNT]; WINDOW_SIZE],
            len: 0,
            write_index: 0,
        }
    }

    pub fn push(&mut self, sample: [i16; FEATURE_COUNT]) {
        self.samples[self.write_index] = sample;
        self.write_index = (self.write_index + 1) % WINDOW_SIZE;
        if self.len < WINDOW_SIZE {
            self.len += 1;
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_full(&self) -> bool {
        self.len == WINDOW_SIZE
    }

    pub fn ordered_sample(&self, logical_index: usize) -> [i16; FEATURE_COUNT] {
        debug_assert!(logical_index < self.len);

        let physical_index = if self.is_full() {
            (self.write_index + logical_index) % WINDOW_SIZE
        } else {
            logical_index
        };

        self.samples[physical_index]
    }
}

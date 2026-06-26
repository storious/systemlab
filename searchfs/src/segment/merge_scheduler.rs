#[derive(Debug, Clone, Copy)]
pub struct MergeScheduler {
    max_segments: usize,
}

impl MergeScheduler {
    pub fn new(max_segments: usize) -> Self {
        Self { max_segments }
    }

    pub fn disabled() -> Self {
        Self {
            max_segments: usize::MAX,
        }
    }

    pub fn should_merge(&self, segment_count: usize) -> bool {
        segment_count > self.max_segments
    }

    pub fn max_segments(&self) -> usize {
        self.max_segments
    }
}

impl Default for MergeScheduler {
    fn default() -> Self {
        Self::new(8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_scheduler_triggers_when_segment_count_exceeds_limit() {
        let scheduler = MergeScheduler::new(3);

        assert!(!scheduler.should_merge(3));
        assert!(scheduler.should_merge(4));
    }

    #[test]
    fn disabled_merge_scheduler_never_triggers_for_normal_counts() {
        let scheduler = MergeScheduler::disabled();

        assert!(!scheduler.should_merge(100));
    }
}

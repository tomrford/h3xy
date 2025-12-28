use crate::Segment;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HexFile {
    segments: Vec<Segment>,
}

impl HexFile {
    pub fn new() -> Self {
        Self { segments: vec![] }
    }

    pub fn with_segments(segments: Vec<Segment>) -> Self {
        let mut hf = Self { segments };
        hf.normalize();
        hf
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    pub fn into_segments(self) -> Vec<Segment> {
        self.segments
    }

    pub fn add_segment(&mut self, segment: Segment) {
        if segment.is_empty() {
            return;
        }
        self.segments.push(segment);
        self.normalize();
    }

    pub fn min_address(&self) -> Option<u32> {
        self.segments.first().map(|s| s.start_address)
    }

    pub fn max_address(&self) -> Option<u32> {
        self.segments.last().map(|s| s.end_address())
    }

    pub fn total_bytes(&self) -> usize {
        self.segments.iter().map(|s| s.len()).sum()
    }

    pub fn gap_count(&self) -> usize {
        if self.segments.len() <= 1 {
            return 0;
        }
        self.segments
            .windows(2)
            .filter(|w| !w[0].is_contiguous_with(&w[1]))
            .count()
    }

    fn normalize(&mut self) {
        if self.segments.is_empty() {
            return;
        }

        self.segments.sort_by_key(|s| s.start_address);

        let mut merged: Vec<Segment> = Vec::with_capacity(self.segments.len());

        for seg in std::mem::take(&mut self.segments) {
            if let Some(last) = merged.last_mut()
                && last.is_contiguous_with(&seg)
            {
                last.merge(seg);
                continue;
            }
            merged.push(seg);
        }

        self.segments = merged;
    }
}

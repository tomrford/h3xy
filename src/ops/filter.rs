use crate::{HexFile, Range, Segment};

/// Options for fill operations.
#[derive(Debug, Clone)]
pub struct FillOptions {
    /// Pattern to repeat (default: [0xFF])
    pub pattern: Vec<u8>,
    /// If true, overwrites existing data; if false, only fills gaps
    pub overwrite: bool,
}

impl Default for FillOptions {
    fn default() -> Self {
        Self {
            pattern: vec![0xFF],
            overwrite: false,
        }
    }
}

/// Mode for merging files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MergeMode {
    /// New data overwrites existing (opaque)
    #[default]
    Overwrite,
    /// Existing data preserved, new fills gaps (transparent)
    Preserve,
}

/// Options for merge operations.
#[derive(Debug, Clone)]
pub struct MergeOptions {
    pub mode: MergeMode,
    /// Address offset to apply (can be negative)
    pub offset: i64,
    /// Only merge data within this range (applied before offset)
    pub range: Option<Range>,
}

impl Default for MergeOptions {
    fn default() -> Self {
        Self {
            mode: MergeMode::Overwrite,
            offset: 0,
            range: None,
        }
    }
}

impl HexFile {
    /// Keep only data within the specified range. Clips segments that partially overlap.
    pub fn filter_range(&mut self, range: Range) {
        self.filter_ranges(&[range]);
    }

    /// Keep only data within any of the specified ranges.
    pub fn filter_ranges(&mut self, ranges: &[Range]) {
        if ranges.is_empty() {
            self.set_segments(Vec::new());
            return;
        }

        let mut new_segments = Vec::new();

        for segment in self.segments() {
            let seg_range =
                match Range::from_start_end(segment.start_address, segment.end_address()) {
                    Ok(r) => r,
                    Err(_) => continue,
                };

            for range in ranges {
                if let Some(intersection) = seg_range.intersection(range) {
                    let start_offset = (intersection.start() - segment.start_address) as usize;
                    let end_offset = (intersection.end() - segment.start_address) as usize + 1;
                    let data = segment.data[start_offset..end_offset].to_vec();
                    new_segments.push(Segment::new(intersection.start(), data));
                }
            }
        }

        self.set_segments(new_segments);
    }

    /// Remove all data within the specified range. Splits segments if cut is in the middle.
    pub fn cut(&mut self, range: Range) {
        self.cut_ranges(&[range]);
    }

    /// Remove data within multiple ranges.
    pub fn cut_ranges(&mut self, ranges: &[Range]) {
        for range in ranges {
            let mut new_segments = Vec::new();

            for segment in self.segments_mut().drain(..) {
                let seg_start = segment.start_address;
                let seg_end = segment.end_address();

                // No overlap - keep entire segment
                if seg_end < range.start() || seg_start > range.end() {
                    new_segments.push(segment);
                    continue;
                }

                // Keep portion before the cut
                if seg_start < range.start() {
                    let end_offset = (range.start() - seg_start) as usize;
                    let data = segment.data[..end_offset].to_vec();
                    new_segments.push(Segment::new(seg_start, data));
                }

                // Keep portion after the cut
                if seg_end > range.end() {
                    let start_offset = (range.end() - seg_start + 1) as usize;
                    let data = segment.data[start_offset..].to_vec();
                    new_segments.push(Segment::new(range.end() + 1, data));
                }
            }

            self.set_segments(new_segments);
        }
    }

    /// Fill a region with the specified pattern.
    /// By default (overwrite=false), only fills gaps - existing data is preserved.
    pub fn fill(&mut self, range: Range, options: &FillOptions) {
        self.fill_ranges(&[range], options);
    }

    /// Fill multiple regions with the specified pattern.
    pub fn fill_ranges(&mut self, ranges: &[Range], options: &FillOptions) {
        if options.pattern.is_empty() {
            return;
        }

        for range in ranges {
            if options.overwrite {
                // Remove existing data in range, then fill
                self.cut(*range);
            }

            // Generate the fill data
            let len = range.length() as usize;
            let mut data = Vec::with_capacity(len);
            let pattern = &options.pattern;
            for i in 0..len {
                data.push(pattern[i % pattern.len()]);
            }

            // Prepend so existing data takes priority (low priority fill)
            self.prepend_segment(Segment::new(range.start(), data));
        }
    }

    /// Fill all gaps between first and last segment with fill byte.
    /// Result: single contiguous segment.
    /// Returns silently if the span is too large (>= 4GiB).
    pub fn fill_gaps(&mut self, fill_byte: u8) {
        let normalized = self.normalized_lossy();
        let Some(min_addr) = normalized.min_address() else {
            return;
        };
        let Some(max_addr) = normalized.max_address() else {
            return;
        };

        // Compute span in u64 to avoid overflow
        let span = (max_addr as u64) - (min_addr as u64) + 1;
        if span > usize::MAX as u64 {
            return;
        }

        let total_len = span as usize;
        let mut data = vec![fill_byte; total_len];

        // Copy existing data into the buffer
        for segment in normalized.segments() {
            let offset = (segment.start_address - min_addr) as usize;
            data[offset..offset + segment.len()].copy_from_slice(&segment.data);
        }

        self.set_segments(vec![Segment::new(min_addr, data)]);
    }

    /// Merge another file into this one.
    pub fn merge(&mut self, other: &HexFile, options: &MergeOptions) {
        let mut other_filtered = other.clone();

        // Apply range filter if specified
        if let Some(range) = options.range {
            other_filtered.filter_range(range);
        }

        // Apply offset
        if options.offset != 0 {
            other_filtered.offset_addresses(options.offset);
        }

        match options.mode {
            MergeMode::Overwrite => {
                // Other data is high priority - append so it wins
                for segment in other_filtered.into_segments() {
                    self.append_segment(segment);
                }
            }
            MergeMode::Preserve => {
                // Other data is low priority - prepend so existing wins
                for segment in other_filtered.into_segments() {
                    self.prepend_segment(segment);
                }
            }
        }
    }

    /// Add offset to all segment addresses.
    /// Saturates at 0 for negative offsets that would go below 0.
    /// Saturates at u32::MAX for positive offsets that would overflow.
    pub fn offset_addresses(&mut self, offset: i64) {
        for segment in self.segments_mut() {
            segment.start_address = if offset >= 0 {
                segment.start_address.saturating_add(offset as u32)
            } else {
                // Handle i64::MIN safely by computing absolute value in u64
                let abs = offset.unsigned_abs();
                let abs_u32 = if abs > u32::MAX as u64 {
                    u32::MAX
                } else {
                    abs as u32
                };
                segment.start_address.saturating_sub(abs_u32)
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_range_clips_segment() {
        let mut hf = HexFile::with_segments(vec![Segment::new(
            0x1000,
            vec![0x01, 0x02, 0x03, 0x04, 0x05],
        )]);
        hf.filter_range(Range::from_start_end(0x1001, 0x1003).unwrap());

        assert_eq!(hf.segments().len(), 1);
        assert_eq!(hf.segments()[0].start_address, 0x1001);
        assert_eq!(hf.segments()[0].data, vec![0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_filter_range_removes_outside() {
        let mut hf = HexFile::with_segments(vec![
            Segment::new(0x1000, vec![0x01, 0x02]),
            Segment::new(0x2000, vec![0x03, 0x04]),
            Segment::new(0x3000, vec![0x05, 0x06]),
        ]);
        hf.filter_range(Range::from_start_end(0x2000, 0x2FFF).unwrap());

        assert_eq!(hf.segments().len(), 1);
        assert_eq!(hf.segments()[0].start_address, 0x2000);
    }

    #[test]
    fn test_filter_multiple_ranges() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0x01; 0x100])]);
        hf.filter_ranges(&[
            Range::from_start_end(0x1010, 0x101F).unwrap(),
            Range::from_start_end(0x1080, 0x108F).unwrap(),
        ]);

        assert_eq!(hf.segments().len(), 2);
        assert_eq!(hf.segments()[0].start_address, 0x1010);
        assert_eq!(hf.segments()[0].len(), 0x10);
        assert_eq!(hf.segments()[1].start_address, 0x1080);
        assert_eq!(hf.segments()[1].len(), 0x10);
    }

    #[test]
    fn test_cut_splits_segment() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0x01; 0x100])]);
        hf.cut(Range::from_start_end(0x1040, 0x107F).unwrap());

        let norm = hf.normalized().unwrap();
        assert_eq!(norm.segments().len(), 2);
        assert_eq!(norm.segments()[0].start_address, 0x1000);
        assert_eq!(norm.segments()[0].end_address(), 0x103F);
        assert_eq!(norm.segments()[1].start_address, 0x1080);
        assert_eq!(norm.segments()[1].end_address(), 0x10FF);
    }

    #[test]
    fn test_cut_removes_entire_segment() {
        let mut hf = HexFile::with_segments(vec![
            Segment::new(0x1000, vec![0x01; 0x10]),
            Segment::new(0x2000, vec![0x02; 0x10]),
        ]);
        hf.cut(Range::from_start_end(0x1000, 0x100F).unwrap());

        assert_eq!(hf.segments().len(), 1);
        assert_eq!(hf.segments()[0].start_address, 0x2000);
    }

    #[test]
    fn test_fill_creates_segment() {
        let mut hf = HexFile::new();
        hf.fill(
            Range::from_start_length(0x1000, 8).unwrap(),
            &FillOptions::default(),
        );

        assert_eq!(hf.segments().len(), 1);
        assert_eq!(hf.segments()[0].start_address, 0x1000);
        assert_eq!(hf.segments()[0].data, vec![0xFF; 8]);
    }

    #[test]
    fn test_fill_with_pattern() {
        let mut hf = HexFile::new();
        hf.fill(
            Range::from_start_length(0x1000, 8).unwrap(),
            &FillOptions {
                pattern: vec![0xDE, 0xAD, 0xBE, 0xEF],
                overwrite: false,
            },
        );

        assert_eq!(
            hf.segments()[0].data,
            vec![0xDE, 0xAD, 0xBE, 0xEF, 0xDE, 0xAD, 0xBE, 0xEF]
        );
    }

    #[test]
    fn test_fill_gaps() {
        let mut hf = HexFile::with_segments(vec![
            Segment::new(0x1000, vec![0xAA, 0xBB]),
            Segment::new(0x1004, vec![0xCC, 0xDD]),
        ]);
        hf.fill_gaps(0xFF);

        assert_eq!(hf.segments().len(), 1);
        assert_eq!(hf.segments()[0].start_address, 0x1000);
        assert_eq!(
            hf.segments()[0].data,
            vec![0xAA, 0xBB, 0xFF, 0xFF, 0xCC, 0xDD]
        );
    }

    #[test]
    fn test_offset_positive() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0x01])]);
        hf.offset_addresses(0x1000);
        assert_eq!(hf.segments()[0].start_address, 0x2000);
    }

    #[test]
    fn test_offset_negative() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x2000, vec![0x01])]);
        hf.offset_addresses(-0x1000);
        assert_eq!(hf.segments()[0].start_address, 0x1000);
    }

    #[test]
    fn test_offset_saturates_at_zero() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0x01])]);
        hf.offset_addresses(-0x2000);
        assert_eq!(hf.segments()[0].start_address, 0);
    }

    #[test]
    fn test_merge_overwrite() {
        let mut hf1 = HexFile::with_segments(vec![Segment::new(0x1000, vec![0xAA, 0xBB])]);
        let hf2 = HexFile::with_segments(vec![Segment::new(0x1001, vec![0xFF])]);

        hf1.merge(&hf2, &MergeOptions::default());
        let norm = hf1.normalized_lossy();

        assert_eq!(norm.segments()[0].data, vec![0xAA, 0xFF]);
    }

    #[test]
    fn test_merge_preserve() {
        let mut hf1 = HexFile::with_segments(vec![Segment::new(0x1000, vec![0xAA, 0xBB])]);
        let hf2 = HexFile::with_segments(vec![Segment::new(0x1001, vec![0xFF])]);

        hf1.merge(
            &hf2,
            &MergeOptions {
                mode: MergeMode::Preserve,
                ..Default::default()
            },
        );
        let norm = hf1.normalized_lossy();

        assert_eq!(norm.segments()[0].data, vec![0xAA, 0xBB]);
    }

    #[test]
    fn test_merge_with_offset() {
        let mut hf1 = HexFile::with_segments(vec![Segment::new(0x1000, vec![0xAA])]);
        let hf2 = HexFile::with_segments(vec![Segment::new(0x0000, vec![0xBB])]);

        hf1.merge(
            &hf2,
            &MergeOptions {
                offset: 0x2000,
                ..Default::default()
            },
        );
        let norm = hf1.normalized_lossy();

        assert_eq!(norm.segments().len(), 2);
        assert_eq!(norm.segments()[1].start_address, 0x2000);
    }

    // --- Edge case tests ---

    #[test]
    fn test_filter_range_all_removed() {
        let mut hf = HexFile::with_segments(vec![
            Segment::new(0x1000, vec![0x01, 0x02]),
            Segment::new(0x2000, vec![0x03, 0x04]),
        ]);
        hf.filter_range(Range::from_start_end(0x5000, 0x5FFF).unwrap());
        assert!(hf.segments().is_empty());
    }

    #[test]
    fn test_filter_ranges_empty_clears_all() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0x01, 0x02])]);
        hf.filter_ranges(&[]);
        assert!(hf.segments().is_empty());
    }

    #[test]
    fn test_filter_ranges_overlapping() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0x01; 0x20])]);
        hf.filter_ranges(&[
            Range::from_start_end(0x1005, 0x1015).unwrap(),
            Range::from_start_end(0x1010, 0x101A).unwrap(), // overlaps
        ]);
        let norm = hf.normalized_lossy();
        // Should have data from 0x1005 to 0x101A
        assert_eq!(norm.min_address(), Some(0x1005));
        assert_eq!(norm.max_address(), Some(0x101A));
    }

    #[test]
    fn test_cut_head_only() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0x01; 0x10])]);
        hf.cut(Range::from_start_end(0x1000, 0x1003).unwrap());
        assert_eq!(hf.segments()[0].start_address, 0x1004);
        assert_eq!(hf.segments()[0].len(), 0x0C);
    }

    #[test]
    fn test_cut_tail_only() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0x01; 0x10])]);
        hf.cut(Range::from_start_end(0x100C, 0x100F).unwrap());
        assert_eq!(hf.segments()[0].start_address, 0x1000);
        assert_eq!(hf.segments()[0].len(), 0x0C);
    }

    #[test]
    fn test_cut_multiple_ranges_on_single_segment() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0x01; 0x20])]);
        hf.cut_ranges(&[
            Range::from_start_end(0x1004, 0x1007).unwrap(),
            Range::from_start_end(0x1010, 0x1013).unwrap(),
        ]);
        let norm = hf.normalized().unwrap();
        assert_eq!(norm.segments().len(), 3);
    }

    #[test]
    fn test_cut_spanning_multiple_segments() {
        let mut hf = HexFile::with_segments(vec![
            Segment::new(0x1000, vec![0x01; 0x10]),
            Segment::new(0x1020, vec![0x02; 0x10]),
        ]);
        hf.cut(Range::from_start_end(0x1008, 0x1027).unwrap());
        let norm = hf.normalized().unwrap();
        assert_eq!(norm.segments().len(), 2);
        assert_eq!(norm.segments()[0].end_address(), 0x1007);
        assert_eq!(norm.segments()[1].start_address, 0x1028);
    }

    #[test]
    fn test_fill_overwrite_partial() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0xAA; 8])]);
        hf.fill(
            Range::from_start_length(0x1002, 4).unwrap(),
            &FillOptions {
                pattern: vec![0xFF],
                overwrite: true,
            },
        );
        let norm = hf.normalized_lossy();
        assert_eq!(
            norm.segments()[0].data,
            vec![0xAA, 0xAA, 0xFF, 0xFF, 0xFF, 0xFF, 0xAA, 0xAA]
        );
    }

    #[test]
    fn test_fill_gaps_with_overlapping_segments() {
        let mut hf = HexFile::with_segments(vec![
            Segment::new(0x1000, vec![0xAA, 0xBB, 0xCC]),
            Segment::new(0x1001, vec![0xFF]), // overlaps
        ]);
        hf.fill_gaps(0x00);
        let seg = &hf.segments()[0];
        assert_eq!(seg.start_address, 0x1000);
        // normalized_lossy: last wins, so 0x1001 = 0xFF
        assert_eq!(seg.data, vec![0xAA, 0xFF, 0xCC]);
    }

    #[test]
    fn test_fill_gaps_single_segment() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0xAA, 0xBB])]);
        hf.fill_gaps(0xFF);
        assert_eq!(hf.segments().len(), 1);
        assert_eq!(hf.segments()[0].data, vec![0xAA, 0xBB]);
    }

    #[test]
    fn test_merge_with_negative_offset() {
        let mut hf1 = HexFile::with_segments(vec![Segment::new(0x1000, vec![0xAA])]);
        let hf2 = HexFile::with_segments(vec![Segment::new(0x3000, vec![0xBB])]);

        hf1.merge(
            &hf2,
            &MergeOptions {
                offset: -0x1000,
                ..Default::default()
            },
        );
        let norm = hf1.normalized_lossy();
        assert_eq!(norm.segments().len(), 2);
        assert_eq!(norm.segments()[1].start_address, 0x2000);
    }

    #[test]
    fn test_merge_with_range_filter() {
        let mut hf1 = HexFile::new();
        let hf2 = HexFile::with_segments(vec![
            Segment::new(0x1000, vec![0xAA; 0x10]),
            Segment::new(0x2000, vec![0xBB; 0x10]),
        ]);

        hf1.merge(
            &hf2,
            &MergeOptions {
                range: Some(Range::from_start_end(0x2000, 0x2FFF).unwrap()),
                ..Default::default()
            },
        );
        assert_eq!(hf1.segments().len(), 1);
        assert_eq!(hf1.segments()[0].start_address, 0x2000);
    }

    #[test]
    fn test_offset_saturates_at_max() {
        let mut hf = HexFile::with_segments(vec![Segment::new(u32::MAX - 0x100, vec![0x01])]);
        hf.offset_addresses(0x1000);
        assert_eq!(hf.segments()[0].start_address, u32::MAX);
    }

    #[test]
    fn test_offset_i64_min_handled() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0x01])]);
        hf.offset_addresses(i64::MIN);
        assert_eq!(hf.segments()[0].start_address, 0);
    }

    #[test]
    fn test_offset_large_negative() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0x01])]);
        hf.offset_addresses(-0x1_0000_0000_i64); // > u32::MAX
        assert_eq!(hf.segments()[0].start_address, 0);
    }
}

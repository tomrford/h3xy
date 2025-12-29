use std::collections::BTreeMap;

use thiserror::Error;

use crate::Segment;

#[derive(Debug, Error)]
pub enum HexFileError {
    #[error(
        "overlapping segments at address {address:#X}: existing {existing_start:#X}..={existing_end:#X}, new {new_start:#X}..={new_end:#X}"
    )]
    OverlappingSegments {
        address: u32,
        existing_start: u32,
        existing_end: u32,
        new_start: u32,
        new_end: u32,
    },
}

/// A collection of memory segments.
///
/// Segments may overlap. Use `normalized()` or `normalized_lossy()` to resolve overlaps:
/// - `normalized()` errors on overlap
/// - `normalized_lossy()` uses "last wins" - later segments overwrite earlier ones
///
/// Use `append_segment` for high-priority data (wins on overlap).
/// Use `prepend_segment` for low-priority data (loses on overlap).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HexFile {
    segments: Vec<Segment>,
}

impl HexFile {
    pub fn new() -> Self {
        Self { segments: vec![] }
    }

    pub fn with_segments(segments: Vec<Segment>) -> Self {
        Self {
            segments: segments.into_iter().filter(|s| !s.is_empty()).collect(),
        }
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    pub fn segments_mut(&mut self) -> &mut Vec<Segment> {
        &mut self.segments
    }

    pub fn into_segments(self) -> Vec<Segment> {
        self.segments
    }

    pub fn set_segments(&mut self, segments: Vec<Segment>) {
        self.segments = segments;
    }

    /// Add segment with HIGH priority (wins on overlap after normalize).
    pub fn append_segment(&mut self, segment: Segment) {
        if segment.is_empty() {
            return;
        }
        self.segments.push(segment);
    }

    /// Add segment with LOW priority (loses on overlap after normalize).
    pub fn prepend_segment(&mut self, segment: Segment) {
        if segment.is_empty() {
            return;
        }
        self.segments.insert(0, segment);
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn min_address(&self) -> Option<u32> {
        self.segments.iter().map(|s| s.start_address).min()
    }

    pub fn max_address(&self) -> Option<u32> {
        self.segments.iter().map(|s| s.end_address()).max()
    }

    pub fn total_bytes(&self) -> usize {
        self.segments.iter().map(|s| s.len()).sum()
    }

    /// Returns sorted/merged copy. Errors if any segments overlap.
    pub fn normalized(&self) -> Result<HexFile, HexFileError> {
        if self.segments.is_empty() {
            return Ok(HexFile::new());
        }

        let mut sorted: Vec<_> = self.segments.iter().collect();
        sorted.sort_by_key(|s| s.start_address);

        let mut merged: Vec<Segment> = Vec::with_capacity(sorted.len());

        for seg in sorted {
            if let Some(last) = merged.last_mut() {
                if seg.start_address <= last.end_address() {
                    return Err(HexFileError::OverlappingSegments {
                        address: seg.start_address,
                        existing_start: last.start_address,
                        existing_end: last.end_address(),
                        new_start: seg.start_address,
                        new_end: seg.end_address(),
                    });
                }
                if last.is_contiguous_with(seg) {
                    last.data.extend_from_slice(&seg.data);
                    continue;
                }
            }
            merged.push(seg.clone());
        }

        Ok(HexFile { segments: merged })
    }

    /// Returns sorted/merged copy. Later-inserted segments overwrite earlier ones on overlap.
    /// Bytes that would overflow u32 address space are silently dropped.
    pub fn normalized_lossy(&self) -> HexFile {
        if self.segments.is_empty() {
            return HexFile::new();
        }

        // Build sparse byte map: address -> byte value
        // Apply segments in insertion order (last wins)
        let mut byte_map: BTreeMap<u32, u8> = BTreeMap::new();

        for seg in &self.segments {
            for (offset, &byte) in seg.data.iter().enumerate() {
                let Some(addr) = seg.start_address.checked_add(offset as u32) else {
                    break;
                };
                byte_map.insert(addr, byte);
            }
        }

        // Convert back to segments
        segments_from_byte_map(byte_map)
    }

    /// Count gaps between segments (after sorting).
    pub fn gap_count(&self) -> usize {
        if self.segments.len() <= 1 {
            return 0;
        }
        let mut sorted: Vec<_> = self.segments.iter().collect();
        sorted.sort_by_key(|s| s.start_address);
        sorted
            .windows(2)
            .filter(|w| !w[0].is_contiguous_with(w[1]))
            .count()
    }

    // --- Address-based access ---

    /// Read a single byte at address. Returns None if address is not covered by any segment.
    pub fn read_byte(&self, addr: u32) -> Option<u8> {
        for seg in &self.segments {
            if addr >= seg.start_address && addr <= seg.end_address() {
                let offset = (addr - seg.start_address) as usize;
                return Some(seg.data[offset]);
            }
        }
        None
    }

    /// Read bytes from address range. Returns None for gaps.
    pub fn read_bytes(&self, addr: u32, len: usize) -> Vec<Option<u8>> {
        (0..len)
            .map(|i| {
                let a = addr.checked_add(i as u32)?;
                self.read_byte(a)
            })
            .collect()
    }

    /// Read bytes from address range. Returns None if any address in range is not covered.
    pub fn read_bytes_contiguous(&self, addr: u32, len: usize) -> Option<Vec<u8>> {
        self.read_bytes(addr, len).into_iter().collect()
    }

    /// Write bytes at address. Creates new segment, will overlap with existing data.
    /// Use normalized_lossy() after to merge and resolve overlaps.
    pub fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        self.segments.push(Segment::new(addr, data.to_vec()));
    }
}

fn segments_from_byte_map(byte_map: BTreeMap<u32, u8>) -> HexFile {
    if byte_map.is_empty() {
        return HexFile::new();
    }

    let mut segments = Vec::new();
    let mut iter = byte_map.into_iter();
    let (first_addr, first_byte) = iter.next().unwrap();
    let mut current = Segment::new(first_addr, vec![first_byte]);

    for (addr, byte) in iter {
        if current.end_address() + 1 == addr {
            current.data.push(byte);
        } else {
            segments.push(current);
            current = Segment::new(addr, vec![byte]);
        }
    }
    segments.push(current);

    HexFile { segments }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalized_merges_contiguous() {
        let hf = HexFile::with_segments(vec![
            Segment::new(0x100, vec![0x01, 0x02]),
            Segment::new(0x102, vec![0x03, 0x04]),
        ]);
        let norm = hf.normalized().unwrap();
        assert_eq!(norm.segments.len(), 1);
        assert_eq!(norm.segments[0].start_address, 0x100);
        assert_eq!(norm.segments[0].data, vec![0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_normalized_preserves_gaps() {
        let hf = HexFile::with_segments(vec![
            Segment::new(0x100, vec![0x01, 0x02]),
            Segment::new(0x200, vec![0x03, 0x04]),
        ]);
        let norm = hf.normalized().unwrap();
        assert_eq!(norm.segments.len(), 2);
    }

    #[test]
    fn test_normalized_errors_on_overlap() {
        let hf = HexFile::with_segments(vec![
            Segment::new(0x100, vec![0x01, 0x02, 0x03]),
            Segment::new(0x101, vec![0xFF]),
        ]);
        assert!(matches!(
            hf.normalized(),
            Err(HexFileError::OverlappingSegments { .. })
        ));
    }

    #[test]
    fn test_normalized_lossy_last_wins() {
        let hf = HexFile::with_segments(vec![
            Segment::new(0x100, vec![0x01, 0x02, 0x03]),
            Segment::new(0x101, vec![0xFF]),
        ]);
        let norm = hf.normalized_lossy();
        assert_eq!(norm.segments.len(), 1);
        assert_eq!(norm.segments[0].data, vec![0x01, 0xFF, 0x03]);
    }

    #[test]
    fn test_read_byte() {
        let hf = HexFile::with_segments(vec![Segment::new(0x100, vec![0xAA, 0xBB, 0xCC])]);
        assert_eq!(hf.read_byte(0x100), Some(0xAA));
        assert_eq!(hf.read_byte(0x101), Some(0xBB));
        assert_eq!(hf.read_byte(0x102), Some(0xCC));
        assert_eq!(hf.read_byte(0x103), None);
        assert_eq!(hf.read_byte(0x0FF), None);
    }

    #[test]
    fn test_read_bytes_with_gaps() {
        let hf = HexFile::with_segments(vec![
            Segment::new(0x100, vec![0xAA]),
            Segment::new(0x102, vec![0xCC]),
        ]);
        let result = hf.read_bytes(0x100, 3);
        assert_eq!(result, vec![Some(0xAA), None, Some(0xCC)]);
    }

    #[test]
    fn test_read_bytes_contiguous() {
        let hf = HexFile::with_segments(vec![Segment::new(0x100, vec![0xAA, 0xBB, 0xCC])]);
        assert_eq!(
            hf.read_bytes_contiguous(0x100, 3),
            Some(vec![0xAA, 0xBB, 0xCC])
        );
        assert_eq!(hf.read_bytes_contiguous(0x100, 4), None);
    }

    #[test]
    fn test_write_bytes() {
        let mut hf = HexFile::new();
        hf.write_bytes(0x100, &[0x01, 0x02]);
        hf.write_bytes(0x101, &[0xFF]); // overlaps
        let norm = hf.normalized_lossy();
        assert_eq!(norm.segments[0].data, vec![0x01, 0xFF]);
    }

    #[test]
    fn test_sorted_order() {
        let hf = HexFile::with_segments(vec![
            Segment::new(0x300, vec![0x03]),
            Segment::new(0x100, vec![0x01]),
            Segment::new(0x200, vec![0x02]),
        ]);
        let norm = hf.normalized().unwrap();
        assert_eq!(norm.segments[0].start_address, 0x100);
        assert_eq!(norm.segments[1].start_address, 0x200);
        assert_eq!(norm.segments[2].start_address, 0x300);
    }
}

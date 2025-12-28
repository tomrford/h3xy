use super::OpsError;
use crate::{HexFile, Segment};

/// Mode for byte swapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapMode {
    /// Swap pairs: AA BB → BB AA
    Word,
    /// Swap quads: AA BB CC DD → DD CC BB AA
    DWord,
}

impl SwapMode {
    fn size(&self) -> usize {
        match self {
            SwapMode::Word => 2,
            SwapMode::DWord => 4,
        }
    }
}

/// Options for alignment operations.
#[derive(Debug, Clone)]
pub struct AlignOptions {
    /// Must be a power of 2
    pub alignment: u32,
    /// Byte to use for padding (default 0xFF)
    pub fill_byte: u8,
    /// Also align segment lengths
    pub align_length: bool,
}

impl Default for AlignOptions {
    fn default() -> Self {
        Self {
            alignment: 4,
            fill_byte: 0xFF,
            align_length: false,
        }
    }
}

fn is_power_of_two(n: u32) -> bool {
    n > 0 && (n & (n - 1)) == 0
}

fn align_down(addr: u32, alignment: u32) -> u32 {
    addr & !(alignment - 1)
}

fn align_up(len: u32, alignment: u32) -> u32 {
    (len + alignment - 1) & !(alignment - 1)
}

impl HexFile {
    /// Align all segment start addresses to multiples of alignment.
    /// Prepends fill bytes as needed. Optionally aligns lengths too.
    pub fn align(&mut self, options: &AlignOptions) -> Result<(), OpsError> {
        if !is_power_of_two(options.alignment) {
            return Err(OpsError::InvalidAlignment(options.alignment));
        }

        for segment in self.segments_mut() {
            let aligned_start = align_down(segment.start_address, options.alignment);

            if aligned_start < segment.start_address {
                let prepend_count = (segment.start_address - aligned_start) as usize;
                let mut new_data = vec![options.fill_byte; prepend_count];
                new_data.append(&mut segment.data);
                segment.data = new_data;
                segment.start_address = aligned_start;
            }

            if options.align_length {
                let current_len = segment.data.len() as u32;
                let aligned_len = align_up(current_len, options.alignment);
                if aligned_len > current_len {
                    segment.data.extend(
                        std::iter::repeat_n(options.fill_byte, (aligned_len - current_len) as usize),
                    );
                }
            }
        }

        Ok(())
    }

    /// Split any segment larger than max_size into multiple segments.
    pub fn split(&mut self, max_size: u32) {
        if max_size == 0 {
            return;
        }

        let mut new_segments: Vec<Segment> = Vec::new();
        let max_size_usize = max_size as usize;

        for segment in self.segments_mut().drain(..) {
            if segment.len() <= max_size_usize {
                new_segments.push(segment);
                continue;
            }

            let mut addr = segment.start_address;
            for chunk in segment.data.chunks(max_size_usize) {
                new_segments.push(Segment::new(addr, chunk.to_vec()));
                addr += chunk.len() as u32;
            }
        }

        self.set_segments(new_segments);
    }

    /// Swap bytes within all segments.
    pub fn swap_bytes(&mut self, mode: SwapMode) -> Result<(), OpsError> {
        let size = mode.size();

        for segment in self.segments_mut() {
            if segment.data.len() % size != 0 {
                return Err(OpsError::LengthNotMultiple {
                    length: segment.data.len(),
                    expected: size,
                    operation: format!("{mode:?} swap"),
                });
            }

            for chunk in segment.data.chunks_exact_mut(size) {
                chunk.reverse();
            }
        }

        Ok(())
    }

    /// Multiply all addresses by factor.
    pub fn scale_addresses(&mut self, factor: u32) {
        for segment in self.segments_mut() {
            segment.start_address = segment.start_address.saturating_mul(factor);
        }
    }

    /// Divide all addresses by divisor. Errors if any address not evenly divisible.
    /// If validation fails, no segments are modified (transactional).
    pub fn unscale_addresses(&mut self, divisor: u32) -> Result<(), OpsError> {
        if divisor == 0 {
            return Err(OpsError::AddressNotDivisible {
                address: 0,
                divisor: 0,
            });
        }

        // First pass: validate all addresses
        for segment in self.segments() {
            if segment.start_address % divisor != 0 {
                return Err(OpsError::AddressNotDivisible {
                    address: segment.start_address,
                    divisor,
                });
            }
        }

        // Second pass: apply mutation
        for segment in self.segments_mut() {
            segment.start_address /= divisor;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_prepends_fill() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1001, vec![0xAA, 0xBB])]);
        hf.align(&AlignOptions {
            alignment: 4,
            fill_byte: 0xFF,
            align_length: false,
        })
        .unwrap();

        assert_eq!(hf.segments()[0].start_address, 0x1000);
        assert_eq!(hf.segments()[0].data, vec![0xFF, 0xAA, 0xBB]);
    }

    #[test]
    fn test_align_with_length() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1001, vec![0xAA, 0xBB])]);
        hf.align(&AlignOptions {
            alignment: 4,
            fill_byte: 0xFF,
            align_length: true,
        })
        .unwrap();

        assert_eq!(hf.segments()[0].start_address, 0x1000);
        // 1 prepended + 2 data + 1 appended = 4
        assert_eq!(hf.segments()[0].len(), 4);
        assert_eq!(hf.segments()[0].data, vec![0xFF, 0xAA, 0xBB, 0xFF]);
    }

    #[test]
    fn test_align_already_aligned() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0xAA; 8])]);
        hf.align(&AlignOptions {
            alignment: 4,
            fill_byte: 0xFF,
            align_length: true,
        })
        .unwrap();

        assert_eq!(hf.segments()[0].start_address, 0x1000);
        assert_eq!(hf.segments()[0].len(), 8);
    }

    #[test]
    fn test_align_invalid_alignment() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0xAA])]);
        let result = hf.align(&AlignOptions {
            alignment: 3, // not power of 2
            fill_byte: 0xFF,
            align_length: false,
        });
        assert!(matches!(result, Err(OpsError::InvalidAlignment(3))));
    }

    #[test]
    fn test_split_segments() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0xAA; 10])]);
        hf.split(4);

        assert_eq!(hf.segments().len(), 3);
        assert_eq!(hf.segments()[0].start_address, 0x1000);
        assert_eq!(hf.segments()[0].len(), 4);
        assert_eq!(hf.segments()[1].start_address, 0x1004);
        assert_eq!(hf.segments()[1].len(), 4);
        assert_eq!(hf.segments()[2].start_address, 0x1008);
        assert_eq!(hf.segments()[2].len(), 2);
    }

    #[test]
    fn test_swap_word() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0xAA, 0xBB, 0xCC, 0xDD])]);
        hf.swap_bytes(SwapMode::Word).unwrap();

        assert_eq!(hf.segments()[0].data, vec![0xBB, 0xAA, 0xDD, 0xCC]);
    }

    #[test]
    fn test_swap_dword() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0xAA, 0xBB, 0xCC, 0xDD])]);
        hf.swap_bytes(SwapMode::DWord).unwrap();

        assert_eq!(hf.segments()[0].data, vec![0xDD, 0xCC, 0xBB, 0xAA]);
    }

    #[test]
    fn test_swap_odd_length_error() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0xAA, 0xBB, 0xCC])]);
        let result = hf.swap_bytes(SwapMode::Word);

        assert!(matches!(result, Err(OpsError::LengthNotMultiple { .. })));
    }

    #[test]
    fn test_scale_addresses() {
        let mut hf = HexFile::with_segments(vec![
            Segment::new(0x1000, vec![0xAA]),
            Segment::new(0x2000, vec![0xBB]),
        ]);
        hf.scale_addresses(2);

        assert_eq!(hf.segments()[0].start_address, 0x2000);
        assert_eq!(hf.segments()[1].start_address, 0x4000);
    }

    #[test]
    fn test_unscale_addresses() {
        let mut hf = HexFile::with_segments(vec![
            Segment::new(0x2000, vec![0xAA]),
            Segment::new(0x4000, vec![0xBB]),
        ]);
        hf.unscale_addresses(2).unwrap();

        assert_eq!(hf.segments()[0].start_address, 0x1000);
        assert_eq!(hf.segments()[1].start_address, 0x2000);
    }

    #[test]
    fn test_unscale_not_divisible() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1001, vec![0xAA])]);
        let result = hf.unscale_addresses(2);

        assert!(matches!(
            result,
            Err(OpsError::AddressNotDivisible {
                address: 0x1001,
                divisor: 2
            })
        ));
    }

    // --- Edge case tests ---

    #[test]
    fn test_align_causes_overlap() {
        let mut hf = HexFile::with_segments(vec![
            Segment::new(0x1001, vec![0xAA]),
            Segment::new(0x1003, vec![0xBB]),
        ]);
        hf.align(&AlignOptions {
            alignment: 4,
            fill_byte: 0xFF,
            align_length: false,
        })
        .unwrap();
        // Both now start at 0x1000 - overlap
        assert!(hf.normalized().is_err());
        // But normalized_lossy should work (last wins)
        let norm = hf.normalized_lossy();
        assert_eq!(norm.segments().len(), 1);
    }

    #[test]
    fn test_align_with_alignment_1() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1001, vec![0xAA, 0xBB])]);
        hf.align(&AlignOptions {
            alignment: 1,
            fill_byte: 0xFF,
            align_length: true,
        })
        .unwrap();
        // No change expected
        assert_eq!(hf.segments()[0].start_address, 0x1001);
        assert_eq!(hf.segments()[0].len(), 2);
    }

    #[test]
    fn test_split_zero_size_noop() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0xAA; 10])]);
        hf.split(0);
        assert_eq!(hf.segments().len(), 1);
    }

    #[test]
    fn test_split_larger_than_segment() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0xAA; 4])]);
        hf.split(100);
        assert_eq!(hf.segments().len(), 1);
    }

    #[test]
    fn test_swap_multiple_segments() {
        let mut hf = HexFile::with_segments(vec![
            Segment::new(0x1000, vec![0x01, 0x02]),
            Segment::new(0x2000, vec![0x03, 0x04]),
        ]);
        hf.swap_bytes(SwapMode::Word).unwrap();
        assert_eq!(hf.segments()[0].data, vec![0x02, 0x01]);
        assert_eq!(hf.segments()[1].data, vec![0x04, 0x03]);
    }

    #[test]
    fn test_swap_dword_larger_buffer() {
        let mut hf = HexFile::with_segments(vec![Segment::new(
            0x1000,
            vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
        )]);
        hf.swap_bytes(SwapMode::DWord).unwrap();
        assert_eq!(
            hf.segments()[0].data,
            vec![0x04, 0x03, 0x02, 0x01, 0x08, 0x07, 0x06, 0x05]
        );
    }

    #[test]
    fn test_scale_unscale_roundtrip() {
        let mut hf = HexFile::with_segments(vec![
            Segment::new(0x1000, vec![0xAA]),
            Segment::new(0x2000, vec![0xBB]),
        ]);
        let original = hf.clone();
        hf.scale_addresses(4);
        hf.unscale_addresses(4).unwrap();
        assert_eq!(hf.segments()[0].start_address, original.segments()[0].start_address);
        assert_eq!(hf.segments()[1].start_address, original.segments()[1].start_address);
    }

    #[test]
    fn test_unscale_transactional() {
        let mut hf = HexFile::with_segments(vec![
            Segment::new(0x2000, vec![0xAA]), // divisible by 2
            Segment::new(0x3001, vec![0xBB]), // NOT divisible by 2
        ]);
        let original_first = hf.segments()[0].start_address;
        let result = hf.unscale_addresses(2);
        assert!(result.is_err());
        // First segment should NOT have been modified
        assert_eq!(hf.segments()[0].start_address, original_first);
    }

    #[test]
    fn test_scale_saturation() {
        let mut hf = HexFile::with_segments(vec![Segment::new(u32::MAX / 2 + 1, vec![0xAA])]);
        hf.scale_addresses(3);
        assert_eq!(hf.segments()[0].start_address, u32::MAX);
    }

    #[test]
    fn test_unscale_by_zero() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0xAA])]);
        let result = hf.unscale_addresses(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_align_then_split() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1001, vec![0xAA; 15])]);
        hf.align(&AlignOptions {
            alignment: 4,
            fill_byte: 0xFF,
            align_length: true,
        })
        .unwrap();
        // After align: start=0x1000, len=16 (1 prepend + 15 data)
        // Actually: 0x1001 aligns down to 0x1000, prepend 1. length 16, aligned to 4 = 16.
        hf.split(8);
        assert_eq!(hf.segments().len(), 2);
        assert_eq!(hf.segments()[0].start_address, 0x1000);
        assert_eq!(hf.segments()[0].len(), 8);
        assert_eq!(hf.segments()[1].start_address, 0x1008);
        assert_eq!(hf.segments()[1].len(), 8);
    }
}

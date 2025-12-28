use std::str::FromStr;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RangeError {
    #[error("invalid range format: {0}")]
    InvalidFormat(String),

    #[error("invalid number: {0}")]
    InvalidNumber(String),

    #[error("range start ({start:#X}) exceeds end ({end:#X})")]
    StartExceedsEnd { start: u32, end: u32 },

    #[error("zero length range at {start:#X}")]
    ZeroLength { start: u32 },
}

/// A memory address range, specified either as start+length or start-end (inclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    start: u32,
    end: u32, // inclusive
}

impl Range {
    /// Create range from start address and length.
    pub fn from_start_length(start: u32, length: u32) -> Result<Self, RangeError> {
        if length == 0 {
            return Err(RangeError::ZeroLength { start });
        }
        let end = start
            .checked_add(length - 1)
            .ok_or_else(|| RangeError::InvalidFormat("address overflow".to_string()))?;
        Ok(Self { start, end })
    }

    /// Create range from start and end addresses (inclusive).
    pub fn from_start_end(start: u32, end: u32) -> Result<Self, RangeError> {
        if start > end {
            return Err(RangeError::StartExceedsEnd { start, end });
        }
        // Reject full 4GiB range as length would overflow u32
        if start == 0 && end == u32::MAX {
            return Err(RangeError::InvalidFormat(
                "range spans entire 4GiB address space".to_string(),
            ));
        }
        Ok(Self { start, end })
    }

    pub fn start(&self) -> u32 {
        self.start
    }

    pub fn end(&self) -> u32 {
        self.end
    }

    pub fn length(&self) -> u32 {
        self.end - self.start + 1
    }

    pub fn contains(&self, addr: u32) -> bool {
        addr >= self.start && addr <= self.end
    }

    pub fn overlaps(&self, other: &Range) -> bool {
        self.start <= other.end && other.start <= self.end
    }

    /// Return the intersection of two ranges, if they overlap.
    pub fn intersection(&self, other: &Range) -> Option<Range> {
        if !self.overlaps(other) {
            return None;
        }
        Some(Range {
            start: self.start.max(other.start),
            end: self.end.min(other.end),
        })
    }
}

/// Parse a number from decimal, hex (0x), or binary (0b or trailing b).
fn parse_number(s: &str) -> Result<u32, RangeError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(RangeError::InvalidNumber("empty string".to_string()));
    }

    let (radix, digits) = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X"))
    {
        (16, hex)
    } else if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        (2, bin)
    } else if let Some(bin) = s.strip_suffix('b').or_else(|| s.strip_suffix('B')) {
        (2, bin)
    } else {
        (10, s)
    };

    u32::from_str_radix(digits, radix).map_err(|e| RangeError::InvalidNumber(e.to_string()))
}

impl FromStr for Range {
    type Err = RangeError;

    /// Parse range from string.
    /// Formats:
    /// - "start,length" (e.g., "0x1000,0x200")
    /// - "start-end" (e.g., "0x1000-0x11FF")
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((start_str, len_str)) = s.split_once(',') {
            let start = parse_number(start_str)?;
            let length = parse_number(len_str)?;
            Range::from_start_length(start, length)
        } else if let Some((start_str, end_str)) = s.split_once('-') {
            let start = parse_number(start_str)?;
            let end = parse_number(end_str)?;
            Range::from_start_end(start, end)
        } else {
            Err(RangeError::InvalidFormat(format!(
                "expected 'start,length' or 'start-end', got '{s}'"
            )))
        }
    }
}

/// Parse multiple ranges separated by ':'.
pub fn parse_ranges(s: &str) -> Result<Vec<Range>, RangeError> {
    s.split(':').map(|part| part.parse()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_start_length() {
        let r = Range::from_start_length(0x1000, 0x200).unwrap();
        assert_eq!(r.start(), 0x1000);
        assert_eq!(r.end(), 0x11FF);
        assert_eq!(r.length(), 0x200);
    }

    #[test]
    fn test_from_start_end() {
        let r = Range::from_start_end(0x1000, 0x11FF).unwrap();
        assert_eq!(r.start(), 0x1000);
        assert_eq!(r.end(), 0x11FF);
        assert_eq!(r.length(), 0x200);
    }

    #[test]
    fn test_contains() {
        let r = Range::from_start_end(0x1000, 0x1FFF).unwrap();
        assert!(r.contains(0x1000));
        assert!(r.contains(0x1500));
        assert!(r.contains(0x1FFF));
        assert!(!r.contains(0x0FFF));
        assert!(!r.contains(0x2000));
    }

    #[test]
    fn test_overlaps() {
        let r1 = Range::from_start_end(0x1000, 0x1FFF).unwrap();
        let r2 = Range::from_start_end(0x1800, 0x2800).unwrap();
        let r3 = Range::from_start_end(0x2000, 0x3000).unwrap();
        let r4 = Range::from_start_end(0x0500, 0x0FFF).unwrap();

        assert!(r1.overlaps(&r2)); // overlap at 0x1800-0x1FFF
        assert!(!r1.overlaps(&r3)); // adjacent but not overlapping
        assert!(!r1.overlaps(&r4)); // no overlap
    }

    #[test]
    fn test_intersection() {
        let r1 = Range::from_start_end(0x1000, 0x1FFF).unwrap();
        let r2 = Range::from_start_end(0x1800, 0x2800).unwrap();

        let i = r1.intersection(&r2).unwrap();
        assert_eq!(i.start(), 0x1800);
        assert_eq!(i.end(), 0x1FFF);

        let r3 = Range::from_start_end(0x2000, 0x3000).unwrap();
        assert!(r1.intersection(&r3).is_none());
    }

    #[test]
    fn test_parse_start_length_hex() {
        let r: Range = "0x1000,0x200".parse().unwrap();
        assert_eq!(r.start(), 0x1000);
        assert_eq!(r.end(), 0x11FF);
    }

    #[test]
    fn test_parse_start_end_hex() {
        let r: Range = "0x1000-0x11FF".parse().unwrap();
        assert_eq!(r.start(), 0x1000);
        assert_eq!(r.end(), 0x11FF);
    }

    #[test]
    fn test_parse_decimal() {
        let r: Range = "4096,512".parse().unwrap();
        assert_eq!(r.start(), 4096);
        assert_eq!(r.length(), 512);
    }

    #[test]
    fn test_parse_binary() {
        let r: Range = "0b1000,0b100".parse().unwrap();
        assert_eq!(r.start(), 8);
        assert_eq!(r.length(), 4);

        let r2: Range = "1000b,100b".parse().unwrap();
        assert_eq!(r2.start(), 8);
        assert_eq!(r2.length(), 4);
    }

    #[test]
    fn test_parse_ranges_multiple() {
        let ranges = parse_ranges("0x1000,0x100:0x2000-0x2FFF").unwrap();
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].start(), 0x1000);
        assert_eq!(ranges[0].end(), 0x10FF);
        assert_eq!(ranges[1].start(), 0x2000);
        assert_eq!(ranges[1].end(), 0x2FFF);
    }

    #[test]
    fn test_zero_length_error() {
        assert!(matches!(
            Range::from_start_length(0x1000, 0),
            Err(RangeError::ZeroLength { .. })
        ));
    }

    #[test]
    fn test_start_exceeds_end_error() {
        assert!(matches!(
            Range::from_start_end(0x2000, 0x1000),
            Err(RangeError::StartExceedsEnd { .. })
        ));
    }

    // --- Edge case tests ---

    #[test]
    fn test_full_4gib_range_rejected() {
        assert!(matches!(
            Range::from_start_end(0, u32::MAX),
            Err(RangeError::InvalidFormat(_))
        ));
    }

    #[test]
    fn test_near_max_range_allowed() {
        // 1 to MAX is allowed (length = MAX)
        let r = Range::from_start_end(1, u32::MAX).unwrap();
        assert_eq!(r.length(), u32::MAX);
    }

    #[test]
    fn test_single_byte_range() {
        let r = Range::from_start_end(0x1000, 0x1000).unwrap();
        assert_eq!(r.length(), 1);
        assert!(r.contains(0x1000));
        assert!(!r.contains(0x1001));
    }

    #[test]
    fn test_parse_u32_max() {
        let r: Range = "0xFFFFFFFF,1".parse().unwrap();
        assert_eq!(r.start(), u32::MAX);
        assert_eq!(r.end(), u32::MAX);
        assert_eq!(r.length(), 1);
    }

    #[test]
    fn test_parse_overflow_number() {
        let result: Result<Range, _> = "0x100000000,1".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_binary() {
        let result: Result<Range, _> = "0b102,1".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_string() {
        let result: Result<Range, _> = "".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_malformed_comma() {
        let result: Result<Range, _> = "0x1000,".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_malformed_dash() {
        let result: Result<Range, _> = "0x1000-".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ranges_single() {
        let ranges = parse_ranges("0x1000,0x100").unwrap();
        assert_eq!(ranges.len(), 1);
    }

    #[test]
    fn test_address_overflow_in_start_length() {
        let result = Range::from_start_length(u32::MAX, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_overlaps_single_byte_boundary() {
        let r1 = Range::from_start_end(0x1000, 0x1000).unwrap();
        let r2 = Range::from_start_end(0x1000, 0x1000).unwrap();
        assert!(r1.overlaps(&r2));

        let r3 = Range::from_start_end(0x1001, 0x1001).unwrap();
        assert!(!r1.overlaps(&r3));
    }
}

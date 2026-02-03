//! Checksum algorithms compatible with HexView.
//!
//! HexView algorithm indices:
//! - 0: ByteSum 16-bit BE
//! - 1: ByteSum 16-bit LE
//! - 2: WordSum BE 16-bit (sum 16-bit BE words)
//! - 3: WordSum LE 16-bit (sum 16-bit LE words)
//! - 4: ByteSum 2's complement
//! - 5: WordSum BE 2's complement
//! - 6: WordSum LE 2's complement
//! - 7: CRC-16 (poly 0x8005)
//! - 9: CRC-32 IEEE
//! - 12: Modular sum (simple byte sum)
//! - 13: CRC-16 CCITT LE (poly 0x1021, init 0xFFFF)
//! - 14: CRC-16 CCITT BE
//! - 17: CRC-16 CCITT LE init 0
//! - 18: CRC-16 CCITT BE init 0

use std::path::PathBuf;

use crate::{HexFile, OpsError, Range, Segment};

/// Target for checksum output.
#[derive(Debug, Clone)]
pub enum ChecksumTarget {
    /// Write to address in hex file
    Address(u32),
    /// Append after last data
    Append,
    /// Prepend before first data
    Prepend,
    /// Write at end, overwriting existing data
    OverwriteEnd,
    /// Write to external file
    File(PathBuf),
}

/// Forced range for checksum calculation, with fill pattern.
#[derive(Debug, Clone)]
pub struct ForcedRange {
    pub range: Range,
    pub pattern: Vec<u8>,
}

/// Checksum algorithm identifier (HexView-compatible).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChecksumAlgorithm {
    ByteSumBe = 0,
    ByteSumLe = 1,
    WordSumBe = 2,
    WordSumLe = 3,
    ByteSumTwosComplement = 4,
    WordSumBeTwosComplement = 5,
    WordSumLeTwosComplement = 6,
    Crc16 = 7,
    Crc32 = 9,
    ModularSum = 12,
    Crc16CcittLe = 13,
    Crc16CcittBe = 14,
    Crc16CcittLeInit0 = 17,
    Crc16CcittBeInit0 = 18,
}

impl ChecksumAlgorithm {
    pub fn from_index(index: u8) -> Result<Self, OpsError> {
        match index {
            0 => Ok(Self::ByteSumBe),
            1 => Ok(Self::ByteSumLe),
            2 => Ok(Self::WordSumBe),
            3 => Ok(Self::WordSumLe),
            4 => Ok(Self::ByteSumTwosComplement),
            5 => Ok(Self::WordSumBeTwosComplement),
            6 => Ok(Self::WordSumLeTwosComplement),
            7 => Ok(Self::Crc16),
            9 => Ok(Self::Crc32),
            12 => Ok(Self::ModularSum),
            13 => Ok(Self::Crc16CcittLe),
            14 => Ok(Self::Crc16CcittBe),
            17 => Ok(Self::Crc16CcittLeInit0),
            18 => Ok(Self::Crc16CcittBeInit0),
            _ => Err(OpsError::UnsupportedChecksumAlgorithm(index)),
        }
    }

    /// Size of the checksum result in bytes.
    pub fn result_size(&self) -> usize {
        match self {
            Self::Crc32 => 4,
            _ => 2,
        }
    }

    /// Whether this algorithm's native output format is little-endian.
    /// Algorithms ending in "LE" or "Le" output little-endian by default.
    pub fn native_little_endian(&self) -> bool {
        matches!(
            self,
            Self::ByteSumLe
                | Self::WordSumLe
                | Self::WordSumLeTwosComplement
                | Self::Crc16CcittLe
                | Self::Crc16CcittLeInit0
        )
    }
}

/// Options for checksum calculation.
#[derive(Debug, Clone)]
pub struct ChecksumOptions {
    pub algorithm: ChecksumAlgorithm,
    pub range: Option<Range>,
    pub little_endian_output: bool,
    pub forced_range: Option<ForcedRange>,
    pub exclude_ranges: Vec<Range>,
    /// When set, this address range is excluded from the checksum calculation.
    /// Used internally when the checksum target is an address within the data.
    pub target_exclude: Option<Range>,
}

impl Default for ChecksumOptions {
    fn default() -> Self {
        Self {
            algorithm: ChecksumAlgorithm::ByteSumBe,
            range: None,
            little_endian_output: false,
            forced_range: None,
            exclude_ranges: Vec::new(),
            target_exclude: None,
        }
    }
}

impl HexFile {
    /// Calculate checksum over the hex file data.
    /// Returns the checksum bytes in the specified endianness.
    /// Uses a normalized (last-wins) snapshot for overlap resolution.
    ///
    /// Output byte order is determined by:
    /// - The algorithm's native format (e.g., ByteSumLe = LE, ByteSumBe = BE)
    /// - XOR'd with `little_endian_output` (true when /CSR is used)
    ///
    /// Example: /CS1 = ByteSumLe (native LE), little_endian_output=false -> LE
    ///          /CSR1 = ByteSumLe (native LE), little_endian_output=true -> BE
    pub fn calculate_checksum(&self, options: &ChecksumOptions) -> Result<Vec<u8>, OpsError> {
        let data = self.collect_data_for_checksum(options)?;

        // Effective endianness: algorithm's native XOR reversed flag
        // /CS uses algorithm's native format, /CSR inverts it
        let use_le = options.algorithm.native_little_endian() ^ options.little_endian_output;

        fn u16_bytes(value: u16, little_endian: bool) -> Vec<u8> {
            if little_endian {
                value.to_le_bytes().to_vec()
            } else {
                value.to_be_bytes().to_vec()
            }
        }

        fn u32_bytes(value: u32, little_endian: bool) -> Vec<u8> {
            if little_endian {
                value.to_le_bytes().to_vec()
            } else {
                value.to_be_bytes().to_vec()
            }
        }

        let result = match options.algorithm {
            ChecksumAlgorithm::ByteSumBe | ChecksumAlgorithm::ByteSumLe => {
                let sum = byte_sum(&data);
                u16_bytes(sum, use_le)
            }
            ChecksumAlgorithm::WordSumBe => {
                let sum = word_sum_be(&data)?;
                u16_bytes(sum, use_le)
            }
            ChecksumAlgorithm::WordSumLe => {
                let sum = word_sum_le(&data)?;
                u16_bytes(sum, use_le)
            }
            ChecksumAlgorithm::ByteSumTwosComplement => {
                let sum = byte_sum(&data);
                let twos = (!sum).wrapping_add(1);
                u16_bytes(twos, use_le)
            }
            ChecksumAlgorithm::WordSumBeTwosComplement => {
                let sum = word_sum_be(&data)?;
                let twos = (!sum).wrapping_add(1);
                u16_bytes(twos, use_le)
            }
            ChecksumAlgorithm::WordSumLeTwosComplement => {
                let sum = word_sum_le(&data)?;
                let twos = (!sum).wrapping_add(1);
                u16_bytes(twos, use_le)
            }
            ChecksumAlgorithm::ModularSum => {
                let sum = byte_sum(&data);
                u16_bytes(sum, use_le)
            }
            ChecksumAlgorithm::Crc16 => {
                let crc = crc16_arc(&data);
                u16_bytes(crc, use_le)
            }
            ChecksumAlgorithm::Crc32 => {
                let crc = crc32_iso_hdlc(&data);
                u32_bytes(crc, use_le)
            }
            ChecksumAlgorithm::Crc16CcittLe | ChecksumAlgorithm::Crc16CcittBe => {
                let crc = crc16_ibm_sdlc(&data);
                u16_bytes(crc, use_le)
            }
            ChecksumAlgorithm::Crc16CcittLeInit0 | ChecksumAlgorithm::Crc16CcittBeInit0 => {
                let crc = crc16_xmodem(&data);
                u16_bytes(crc, use_le)
            }
        };

        Ok(result)
    }

    /// Calculate checksum and write to target.
    pub fn checksum(
        &mut self,
        options: &ChecksumOptions,
        target: &ChecksumTarget,
    ) -> Result<Vec<u8>, OpsError> {
        // When target overwrites existing data, exclude that range from checksum.
        // This matches HexView behavior where the checksum target is not included.
        let mut effective_options = options.clone();
        let size = options.algorithm.result_size() as u32;
        match target {
            ChecksumTarget::Address(addr) => {
                if let Ok(target_range) = Range::from_start_length(*addr, size) {
                    effective_options.target_exclude = Some(target_range);
                }
            }
            ChecksumTarget::OverwriteEnd => {
                // Overwrite end writes at (max_address - size + 1)
                if let Some(end) = self.max_address() {
                    let offset = size.saturating_sub(1);
                    if let Some(write_addr) = end.checked_sub(offset)
                        && let Ok(target_range) = Range::from_start_length(write_addr, size)
                    {
                        effective_options.target_exclude = Some(target_range);
                    }
                }
            }
            _ => {}
        }
        let result = self.calculate_checksum(&effective_options)?;

        match target {
            ChecksumTarget::Address(addr) => {
                self.write_bytes(*addr, &result);
            }
            ChecksumTarget::Append => {
                if let Some(end) = self.max_address() {
                    let addr = end.checked_add(1).ok_or_else(|| {
                        OpsError::AddressOverflow("checksum append overflows u32".into())
                    })?;
                    self.write_bytes(addr, &result);
                }
            }
            ChecksumTarget::Prepend => {
                if let Some(start) = self.min_address() {
                    let new_start = start.checked_sub(result.len() as u32).ok_or_else(|| {
                        OpsError::AddressOverflow("checksum prepend underflows u32".into())
                    })?;
                    self.write_bytes(new_start, &result);
                }
            }
            ChecksumTarget::OverwriteEnd => {
                if let Some(end) = self.max_address() {
                    // Write checksum to overwrite the last N bytes
                    // For N bytes ending at `end`, start address is `end - (N - 1)`
                    let offset = (result.len() as u32).saturating_sub(1);
                    let write_addr = end.checked_sub(offset).ok_or_else(|| {
                        OpsError::AddressOverflow("checksum overwrite underflows u32".into())
                    })?;
                    self.write_bytes(write_addr, &result);
                }
            }
            ChecksumTarget::File(_) => {
                // File output is handled by caller
            }
        }

        Ok(result)
    }

    /// Collect contiguous data for checksum calculation.
    /// If a range is specified, only include data in that range.
    fn collect_data_for_checksum(&self, options: &ChecksumOptions) -> Result<Vec<u8>, OpsError> {
        let normalized = self.normalized_lossy();
        let needs_word_alignment = matches!(
            options.algorithm,
            ChecksumAlgorithm::WordSumBe
                | ChecksumAlgorithm::WordSumLe
                | ChecksumAlgorithm::WordSumBeTwosComplement
                | ChecksumAlgorithm::WordSumLeTwosComplement
        );

        let working = if let Some(forced) = options.forced_range.as_ref() {
            let mut combined = HexFile::new();
            let fill = build_pattern_data(forced.range, &forced.pattern)?;
            combined.append_segment(Segment::new(forced.range.start(), fill));
            for segment in normalized.segments() {
                combined.append_segment(segment.clone());
            }
            combined.normalized_lossy()
        } else {
            normalized
        };

        let effective_range =
            if let Some(r) = options.range {
                Some(r)
            } else if let Some(forced) = options.forced_range.as_ref() {
                Some(forced.range)
            } else if let (Some(min), Some(max)) = (working.min_address(), working.max_address()) {
                Some(Range::from_start_end(min, max).map_err(|e| {
                    OpsError::AddressOverflow(format!("checksum range invalid: {e}"))
                })?)
            } else {
                None
            };

        let Some(range) = effective_range else {
            return Ok(Vec::new());
        };

        let mut excludes = options.exclude_ranges.clone();
        if let Some(target) = options.target_exclude {
            excludes.push(target);
        }
        let excludes = merge_ranges(excludes);
        let include_ranges = subtract_ranges(range, &excludes);
        if include_ranges.is_empty() {
            return Ok(Vec::new());
        }

        let mut cap_u64: u64 = 0;
        let segments = working.segments();
        let has_forced_range = options.forced_range.is_some();

        if has_forced_range {
            for r in &include_ranges {
                cap_u64 = cap_u64.saturating_add(r.length() as u64);
            }
        } else {
            let mut seg_idx = 0usize;
            let mut inc_idx = 0usize;
            while seg_idx < segments.len() && inc_idx < include_ranges.len() {
                let seg = &segments[seg_idx];
                let inc = include_ranges[inc_idx];
                if seg.end_address() < inc.start() {
                    seg_idx += 1;
                    continue;
                }
                if seg.start_address > inc.end() {
                    inc_idx += 1;
                    continue;
                }
                let start = seg.start_address.max(inc.start());
                let end = seg.end_address().min(inc.end());
                cap_u64 = cap_u64.saturating_add((end - start + 1) as u64);
                if seg.end_address() <= inc.end() {
                    seg_idx += 1;
                } else {
                    inc_idx += 1;
                }
            }
        }

        let cap = usize::try_from(cap_u64).map_err(|_| {
            OpsError::AddressOverflow(format!(
                "checksum range length exceeds usize (start={:#X}, end={:#X})",
                range.start(),
                range.end()
            ))
        })?;

        let mut data = Vec::with_capacity(cap);

        let finalize_run = |run_start: u32, run_len: usize| -> Result<(), OpsError> {
            if !needs_word_alignment {
                return Ok(());
            }
            if !run_start.is_multiple_of(2) {
                return Err(OpsError::AddressNotDivisible {
                    address: run_start,
                    divisor: 2,
                });
            }
            if !run_len.is_multiple_of(2) {
                return Err(OpsError::LengthNotMultiple {
                    length: run_len,
                    expected: 2,
                    operation: "checksum word range".to_string(),
                });
            }
            Ok(())
        };

        if has_forced_range {
            for r in &include_ranges {
                let run_len = usize::try_from(r.length()).map_err(|_| {
                    OpsError::AddressOverflow(format!(
                        "checksum range length exceeds usize (start={:#X}, end={:#X})",
                        r.start(),
                        r.end()
                    ))
                })?;
                finalize_run(r.start(), run_len)?;
            }

            let mut seg_idx = 0usize;
            for r in include_ranges {
                let mut addr = r.start();
                while seg_idx < segments.len() && segments[seg_idx].end_address() < addr {
                    seg_idx += 1;
                }
                while addr <= r.end() {
                    let Some(seg) = segments.get(seg_idx) else {
                        let len = (r.end() - addr + 1) as usize;
                        data.resize(data.len() + len, 0xFF);
                        break;
                    };
                    if seg.start_address > r.end() {
                        let len = (r.end() - addr + 1) as usize;
                        data.resize(data.len() + len, 0xFF);
                        break;
                    }
                    if seg.start_address > addr {
                        let gap_end = seg.start_address.saturating_sub(1).min(r.end());
                        let len = (gap_end - addr + 1) as usize;
                        data.resize(data.len() + len, 0xFF);
                        addr = gap_end.saturating_add(1);
                        continue;
                    }

                    let seg_start = addr.max(seg.start_address);
                    let seg_end = seg.end_address().min(r.end());
                    let offset = (seg_start - seg.start_address) as usize;
                    let len = (seg_end - seg_start + 1) as usize;
                    data.extend_from_slice(&seg.data[offset..offset + len]);

                    if seg.end_address() <= seg_end {
                        seg_idx += 1;
                    }
                    if let Some(next_addr) = seg_end.checked_add(1) {
                        addr = next_addr;
                    } else {
                        break;
                    }
                }
            }
        } else {
            let mut run_start: Option<u32> = None;
            let mut run_len: usize = 0;
            let mut prev_end: Option<u32> = None;
            let mut seg_idx = 0usize;
            let mut inc_idx = 0usize;

            while seg_idx < segments.len() && inc_idx < include_ranges.len() {
                let seg = &segments[seg_idx];
                let inc = include_ranges[inc_idx];
                if seg.end_address() < inc.start() {
                    seg_idx += 1;
                    continue;
                }
                if seg.start_address > inc.end() {
                    inc_idx += 1;
                    continue;
                }
                let start = seg.start_address.max(inc.start());
                let end = seg.end_address().min(inc.end());

                if let Some(prev) = prev_end
                    && start != prev.saturating_add(1)
                {
                    if let Some(start_addr) = run_start.take() {
                        finalize_run(start_addr, run_len)?;
                    }
                    run_len = 0;
                }
                if run_start.is_none() {
                    run_start = Some(start);
                }

                let offset = (start - seg.start_address) as usize;
                let len = (end - start + 1) as usize;
                data.extend_from_slice(&seg.data[offset..offset + len]);
                run_len += len;
                prev_end = Some(end);

                if seg.end_address() <= inc.end() {
                    seg_idx += 1;
                } else {
                    inc_idx += 1;
                }
            }

            if let Some(start_addr) = run_start {
                finalize_run(start_addr, run_len)?;
            }
        }

        Ok(data)
    }
}

fn build_pattern_data(range: Range, pattern: &[u8]) -> Result<Vec<u8>, OpsError> {
    let len = usize::try_from(range.length()).map_err(|_| {
        OpsError::AddressOverflow(format!(
            "forced range length exceeds usize (start={:#X}, end={:#X})",
            range.start(),
            range.end()
        ))
    })?;
    if len == 0 {
        return Ok(Vec::new());
    }
    let fill_pattern = if pattern.is_empty() { &[0xFF] } else { pattern };
    let mut data = Vec::with_capacity(len);
    for i in 0..len {
        data.push(fill_pattern[i % fill_pattern.len()]);
    }
    Ok(data)
}

fn merge_ranges(mut ranges: Vec<Range>) -> Vec<Range> {
    ranges.sort_by_key(|r| r.start());
    let mut merged: Vec<Range> = Vec::new();
    for r in ranges {
        if let Some(last) = merged.last_mut() {
            let last_end = last.end();
            let extend = last_end
                .checked_add(1)
                .map(|v| r.start() <= v)
                .unwrap_or(false);
            if r.start() <= last_end || extend {
                let end = last_end.max(r.end());
                *last = Range::from_start_end(last.start(), end).expect("range merge");
                continue;
            }
        }
        merged.push(r);
    }
    merged
}

fn subtract_ranges(range: Range, excludes: &[Range]) -> Vec<Range> {
    if excludes.is_empty() {
        return vec![range];
    }
    let mut out: Vec<Range> = Vec::new();
    let mut cursor = range.start();
    for ex in excludes {
        if ex.end() < cursor {
            continue;
        }
        if ex.start() > range.end() {
            break;
        }
        let ex_start = ex.start().max(range.start());
        let ex_end = ex.end().min(range.end());
        if cursor < ex_start
            && let Ok(r) = Range::from_start_end(cursor, ex_start - 1)
        {
            out.push(r);
        }
        if ex_end == u32::MAX {
            return out;
        }
        cursor = ex_end + 1;
        if cursor > range.end() {
            return out;
        }
    }
    if cursor <= range.end()
        && let Ok(r) = Range::from_start_end(cursor, range.end())
    {
        out.push(r);
    }
    out
}

/// Sum all bytes, wrapping to 16-bit.
fn byte_sum(data: &[u8]) -> u16 {
    data.iter().fold(0u16, |acc, &b| acc.wrapping_add(b as u16))
}

/// Sum 16-bit big-endian words.
fn word_sum_be(data: &[u8]) -> Result<u16, OpsError> {
    if !data.len().is_multiple_of(2) {
        return Err(OpsError::LengthNotMultiple {
            length: data.len(),
            expected: 2,
            operation: "word sum BE".to_string(),
        });
    }
    Ok(data.chunks_exact(2).fold(0u16, |acc, chunk| {
        acc.wrapping_add(u16::from_be_bytes([chunk[0], chunk[1]]))
    }))
}

/// Sum 16-bit little-endian words.
fn word_sum_le(data: &[u8]) -> Result<u16, OpsError> {
    if !data.len().is_multiple_of(2) {
        return Err(OpsError::LengthNotMultiple {
            length: data.len(),
            expected: 2,
            operation: "word sum LE".to_string(),
        });
    }
    Ok(data.chunks_exact(2).fold(0u16, |acc, chunk| {
        acc.wrapping_add(u16::from_le_bytes([chunk[0], chunk[1]]))
    }))
}

/// CRC-16 with poly 0x8005 (CRC-16-ARC/CRC-16-IBM).
fn crc16_arc(data: &[u8]) -> u16 {
    const CRC: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_ARC);
    CRC.checksum(data)
}

/// CRC-32 IEEE (ISO-HDLC).
fn crc32_iso_hdlc(data: &[u8]) -> u32 {
    const CRC: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);
    CRC.checksum(data)
}

/// CRC-16 CCITT with init 0xFFFF (IBM-SDLC, ISO-HDLC).
fn crc16_ibm_sdlc(data: &[u8]) -> u16 {
    const CRC: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);
    CRC.checksum(data)
}

/// CRC-16 CCITT with init 0 (XMODEM).
fn crc16_xmodem(data: &[u8]) -> u16 {
    const CRC: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_XMODEM);
    CRC.checksum(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Segment;

    #[test]
    fn test_byte_sum() {
        assert_eq!(byte_sum(&[0x01, 0x02, 0x03, 0x04]), 0x000A);
        assert_eq!(byte_sum(&[0xFF, 0xFF]), 0x01FE);
        assert_eq!(byte_sum(&[]), 0);
    }

    #[test]
    fn test_byte_sum_overflow() {
        // 257 * 0xFF = 65535 = 0xFFFF (max u16)
        let data = vec![0xFF; 257];
        assert_eq!(byte_sum(&data), 0xFFFF);

        // Test actual wrapping: 258 * 0xFF = 65790, wraps to 65790 - 65536 = 254 = 0x00FE
        let data2 = vec![0xFF; 258];
        assert_eq!(byte_sum(&data2), 0x00FE);
    }

    #[test]
    fn test_word_sum_be() {
        assert_eq!(word_sum_be(&[0x00, 0x01, 0x00, 0x02]).unwrap(), 0x0003);
        assert_eq!(word_sum_be(&[0x12, 0x34, 0x56, 0x78]).unwrap(), 0x68AC);
    }

    #[test]
    fn test_word_sum_le() {
        assert_eq!(word_sum_le(&[0x01, 0x00, 0x02, 0x00]).unwrap(), 0x0003);
        assert_eq!(word_sum_le(&[0x34, 0x12, 0x78, 0x56]).unwrap(), 0x68AC);
    }

    #[test]
    fn test_word_sum_odd_length() {
        assert!(word_sum_be(&[0x01, 0x02, 0x03]).is_err());
        assert!(word_sum_le(&[0x01]).is_err());
    }

    #[test]
    fn test_checksum_word_sum_odd_start_rejects() {
        let hf = HexFile::with_segments(vec![Segment::new(0x1001, vec![0xAA, 0xBB])]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::WordSumBe,
            ..ChecksumOptions::default()
        };
        let result = hf.calculate_checksum(&options);
        assert!(matches!(
            result,
            Err(OpsError::AddressNotDivisible {
                address: 0x1001,
                divisor: 2
            })
        ));
    }

    #[test]
    fn test_checksum_word_sum_odd_length_rejects() {
        let hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0xAA, 0xBB, 0xCC])]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::WordSumLe,
            ..ChecksumOptions::default()
        };
        let result = hf.calculate_checksum(&options);
        assert!(matches!(
            result,
            Err(OpsError::LengthNotMultiple {
                length: 3,
                expected: 2,
                ..
            })
        ));
    }

    #[test]
    fn test_twos_complement() {
        let sum: u16 = 0x1234;
        let twos = (!sum).wrapping_add(1);
        assert_eq!(twos, 0xEDCC);
        assert_eq!(sum.wrapping_add(twos), 0);
    }

    #[test]
    fn test_crc16_arc() {
        // Known test vector: "123456789" -> 0xBB3D
        assert_eq!(crc16_arc(b"123456789"), 0xBB3D);
    }

    #[test]
    fn test_crc32_iso_hdlc() {
        // Known test vector: "123456789" -> 0xCBF43926
        assert_eq!(crc32_iso_hdlc(b"123456789"), 0xCBF43926);
    }

    #[test]
    fn test_crc16_xmodem() {
        // Known test vector: "123456789" -> 0x31C3
        assert_eq!(crc16_xmodem(b"123456789"), 0x31C3);
    }

    #[test]
    fn test_crc16_ibm_sdlc() {
        // Known test vector: "123456789" -> 0x906E
        assert_eq!(crc16_ibm_sdlc(b"123456789"), 0x906E);
    }

    #[test]
    fn test_hexfile_checksum_byte_sum() {
        let hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0x01, 0x02, 0x03, 0x04])]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::ByteSumBe,
            range: None,
            little_endian_output: false,
            ..Default::default()
        };
        let result = hf.calculate_checksum(&options).unwrap();
        assert_eq!(result, vec![0x00, 0x0A]);
    }

    #[test]
    fn test_hexfile_checksum_crc32() {
        let hf = HexFile::with_segments(vec![Segment::new(0x1000, b"123456789".to_vec())]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::Crc32,
            range: None,
            little_endian_output: false,
            ..Default::default()
        };
        let result = hf.calculate_checksum(&options).unwrap();
        assert_eq!(result, vec![0xCB, 0xF4, 0x39, 0x26]);
    }

    #[test]
    fn test_hexfile_checksum_crc32_le() {
        let hf = HexFile::with_segments(vec![Segment::new(0x1000, b"123456789".to_vec())]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::Crc32,
            range: None,
            little_endian_output: true,
            ..Default::default()
        };
        let result = hf.calculate_checksum(&options).unwrap();
        assert_eq!(result, vec![0x26, 0x39, 0xF4, 0xCB]);
    }

    #[test]
    fn test_hexfile_checksum_with_range() {
        let hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0x01, 0x02, 0x03, 0x04])]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::ByteSumBe,
            range: Some(Range::from_start_end(0x1001, 0x1002).unwrap()),
            little_endian_output: false,
            ..Default::default()
        };
        let result = hf.calculate_checksum(&options).unwrap();
        // Only 0x02 + 0x03 = 0x05
        assert_eq!(result, vec![0x00, 0x05]);
    }

    #[test]
    fn test_hexfile_checksum_append() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0x01, 0x02])]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::ByteSumBe,
            range: None,
            little_endian_output: false,
            ..Default::default()
        };
        hf.checksum(&options, &ChecksumTarget::Append).unwrap();

        let norm = hf.normalized_lossy();
        assert_eq!(norm.max_address(), Some(0x1003));
    }

    #[test]
    fn test_hexfile_checksum_append_overflow() {
        let mut hf = HexFile::with_segments(vec![Segment::new(u32::MAX - 1, vec![0x01, 0x02])]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::ByteSumBe,
            range: None,
            little_endian_output: false,
            ..Default::default()
        };
        let result = hf.checksum(&options, &ChecksumTarget::Append);
        assert!(matches!(result, Err(OpsError::AddressOverflow(_))));
    }

    #[test]
    fn test_hexfile_checksum_prepend_underflow() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x0, vec![0x01])]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::ByteSumBe,
            range: None,
            little_endian_output: false,
            ..Default::default()
        };
        let result = hf.checksum(&options, &ChecksumTarget::Prepend);
        assert!(matches!(result, Err(OpsError::AddressOverflow(_))));
    }

    #[test]
    fn test_hexfile_checksum_overwrite_underflow() {
        let mut hf = HexFile::with_segments(vec![Segment::new(0x0, vec![0x01])]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::ByteSumBe,
            range: None,
            little_endian_output: false,
            ..Default::default()
        };
        let result = hf.checksum(&options, &ChecksumTarget::OverwriteEnd);
        assert!(matches!(result, Err(OpsError::AddressOverflow(_))));
    }

    #[test]
    fn test_hexfile_checksum_overwrite_end() {
        // Data at 0x1000-0x1003 (4 bytes), checksum is 2 bytes
        // OverwriteEnd should write at 0x1002-0x1003, excluding those bytes from calculation
        // Sum of 0x01 + 0x02 = 0x03, BE = [0x00, 0x03]
        let mut hf =
            HexFile::with_segments(vec![Segment::new(0x1000, vec![0x01, 0x02, 0x03, 0x04])]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::ByteSumBe,
            range: None,
            little_endian_output: false,
            ..Default::default()
        };
        hf.checksum(&options, &ChecksumTarget::OverwriteEnd)
            .unwrap();

        let norm = hf.normalized_lossy();
        assert_eq!(norm.segments().len(), 1);
        assert_eq!(norm.min_address(), Some(0x1000));
        assert_eq!(norm.max_address(), Some(0x1003)); // Same end address
        // First two bytes unchanged, last two overwritten with checksum (0x0003)
        assert_eq!(norm.segments()[0].data, vec![0x01, 0x02, 0x00, 0x03]);
    }

    #[test]
    fn test_hexfile_checksum_overwrite_end_crc32() {
        // Data at 0x1000-0x1007 (8 bytes), CRC32 is 4 bytes
        // OverwriteEnd should write at 0x1004-0x1007
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0xAA; 8])]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::Crc32,
            range: None,
            little_endian_output: false,
            ..Default::default()
        };
        hf.checksum(&options, &ChecksumTarget::OverwriteEnd)
            .unwrap();

        let norm = hf.normalized_lossy();
        assert_eq!(norm.min_address(), Some(0x1000));
        assert_eq!(norm.max_address(), Some(0x1007)); // Same end address
        // First 4 bytes unchanged
        assert_eq!(&norm.segments()[0].data[..4], &[0xAA, 0xAA, 0xAA, 0xAA]);
    }

    #[test]
    fn test_algorithm_from_index() {
        assert!(ChecksumAlgorithm::from_index(0).is_ok());
        assert!(ChecksumAlgorithm::from_index(9).is_ok());
        assert!(ChecksumAlgorithm::from_index(8).is_err()); // not implemented
        assert!(ChecksumAlgorithm::from_index(10).is_err()); // SHA-1
    }

    #[test]
    fn test_algorithm_result_size() {
        assert_eq!(ChecksumAlgorithm::Crc32.result_size(), 4);
        assert_eq!(ChecksumAlgorithm::ByteSumBe.result_size(), 2);
        assert_eq!(ChecksumAlgorithm::Crc16.result_size(), 2);
    }

    #[test]
    fn test_crc16_arc_empty() {
        assert_eq!(crc16_arc(&[]), 0x0000);
    }

    #[test]
    fn test_crc32_iso_hdlc_empty() {
        assert_eq!(crc32_iso_hdlc(&[]), 0x00000000);
    }

    #[test]
    fn test_crc16_xmodem_empty() {
        assert_eq!(crc16_xmodem(&[]), 0x0000);
    }

    #[test]
    fn test_hexfile_checksum_crc16() {
        let hf = HexFile::with_segments(vec![Segment::new(0x1000, b"123456789".to_vec())]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::Crc16,
            range: None,
            little_endian_output: false,
            ..Default::default()
        };
        let result = hf.calculate_checksum(&options).unwrap();
        assert_eq!(result, vec![0xBB, 0x3D]);
    }

    #[test]
    fn test_hexfile_checksum_crc16_le() {
        let hf = HexFile::with_segments(vec![Segment::new(0x1000, b"123456789".to_vec())]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::Crc16,
            range: None,
            little_endian_output: true,
            ..Default::default()
        };
        let result = hf.calculate_checksum(&options).unwrap();
        assert_eq!(result, vec![0x3D, 0xBB]);
    }

    #[test]
    fn test_hexfile_checksum_crc16_ccitt_le_init_ffff() {
        let hf = HexFile::with_segments(vec![Segment::new(0x1000, b"123456789".to_vec())]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::Crc16CcittLe,
            range: None,
            little_endian_output: false,
            ..Default::default()
        };
        let result = hf.calculate_checksum(&options).unwrap();
        // CRC-16 IBM-SDLC: 0x906E, output forced LE
        assert_eq!(result, vec![0x6E, 0x90]);
    }

    #[test]
    fn test_hexfile_checksum_crc16_ccitt_be_init_ffff() {
        let hf = HexFile::with_segments(vec![Segment::new(0x1000, b"123456789".to_vec())]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::Crc16CcittBe,
            range: None,
            little_endian_output: false,
            ..Default::default()
        };
        let result = hf.calculate_checksum(&options).unwrap();
        // CRC-16 IBM-SDLC: 0x906E, output forced BE
        assert_eq!(result, vec![0x90, 0x6E]);
    }

    #[test]
    fn test_hexfile_checksum_crc16_ccitt_le_init_0() {
        let hf = HexFile::with_segments(vec![Segment::new(0x1000, b"123456789".to_vec())]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::Crc16CcittLeInit0,
            range: None,
            little_endian_output: false,
            ..Default::default()
        };
        let result = hf.calculate_checksum(&options).unwrap();
        // CRC-16 XMODEM: 0x31C3, output forced LE
        assert_eq!(result, vec![0xC3, 0x31]);
    }

    #[test]
    fn test_hexfile_checksum_crc16_ccitt_be_init_0() {
        let hf = HexFile::with_segments(vec![Segment::new(0x1000, b"123456789".to_vec())]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::Crc16CcittBeInit0,
            range: None,
            little_endian_output: false,
            ..Default::default()
        };
        let result = hf.calculate_checksum(&options).unwrap();
        // CRC-16 XMODEM: 0x31C3, output forced BE
        assert_eq!(result, vec![0x31, 0xC3]);
    }

    #[test]
    fn test_hexfile_checksum_crc_empty_data() {
        let hf = HexFile::new();
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::Crc32,
            range: None,
            little_endian_output: false,
            ..Default::default()
        };
        let result = hf.calculate_checksum(&options).unwrap();
        assert_eq!(result, vec![0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_hexfile_checksum_crc16_with_range() {
        let hf = HexFile::with_segments(vec![Segment::new(0x1000, b"0123456789".to_vec())]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::Crc16,
            range: Some(Range::from_start_end(0x1001, 0x1009).unwrap()),
            little_endian_output: false,
            ..Default::default()
        };
        let result = hf.calculate_checksum(&options).unwrap();
        // Range extracts "123456789"
        assert_eq!(result, vec![0xBB, 0x3D]);
    }

    #[test]
    fn test_hexfile_checksum_forced_range_fill() {
        let hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0x01, 0x02])]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::ByteSumBe,
            range: None,
            little_endian_output: false,
            forced_range: Some(ForcedRange {
                range: Range::from_start_end(0x1000, 0x1003).unwrap(),
                pattern: vec![0xFF],
            }),
            exclude_ranges: Vec::new(),
            target_exclude: None,
        };
        let result = hf.calculate_checksum(&options).unwrap();
        // 0x01 + 0x02 + 0xFF + 0xFF = 0x0201
        assert_eq!(result, vec![0x02, 0x01]);
    }

    #[test]
    fn test_hexfile_checksum_exclude_ranges() {
        let hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0x01, 0x02, 0x03, 0x04])]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::ByteSumBe,
            range: Some(Range::from_start_end(0x1000, 0x1003).unwrap()),
            little_endian_output: false,
            forced_range: None,
            exclude_ranges: vec![Range::from_start_end(0x1001, 0x1002).unwrap()],
            target_exclude: None,
        };
        let result = hf.calculate_checksum(&options).unwrap();
        // 0x01 + 0x04 = 0x05
        assert_eq!(result, vec![0x00, 0x05]);
    }
}

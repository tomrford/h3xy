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

use crate::{HexFile, OpsError, Range};

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
}

/// Options for checksum calculation.
#[derive(Debug, Clone)]
pub struct ChecksumOptions {
    pub algorithm: ChecksumAlgorithm,
    pub range: Option<Range>,
    pub little_endian_output: bool,
}

impl HexFile {
    /// Calculate checksum over the hex file data.
    /// Returns the checksum bytes in the specified endianness.
    pub fn calculate_checksum(&self, options: &ChecksumOptions) -> Result<Vec<u8>, OpsError> {
        let data = self.collect_data_for_checksum(options.range)?;

        let result = match options.algorithm {
            ChecksumAlgorithm::ByteSumBe => {
                let sum = byte_sum(&data);
                if options.little_endian_output {
                    sum.to_le_bytes().to_vec()
                } else {
                    sum.to_be_bytes().to_vec()
                }
            }
            ChecksumAlgorithm::ByteSumLe => {
                let sum = byte_sum(&data);
                if options.little_endian_output {
                    sum.to_le_bytes().to_vec()
                } else {
                    sum.to_be_bytes().to_vec()
                }
            }
            ChecksumAlgorithm::WordSumBe => {
                let sum = word_sum_be(&data)?;
                if options.little_endian_output {
                    sum.to_le_bytes().to_vec()
                } else {
                    sum.to_be_bytes().to_vec()
                }
            }
            ChecksumAlgorithm::WordSumLe => {
                let sum = word_sum_le(&data)?;
                if options.little_endian_output {
                    sum.to_le_bytes().to_vec()
                } else {
                    sum.to_be_bytes().to_vec()
                }
            }
            ChecksumAlgorithm::ByteSumTwosComplement => {
                let sum = byte_sum(&data);
                let twos = (!sum).wrapping_add(1);
                if options.little_endian_output {
                    twos.to_le_bytes().to_vec()
                } else {
                    twos.to_be_bytes().to_vec()
                }
            }
            ChecksumAlgorithm::WordSumBeTwosComplement => {
                let sum = word_sum_be(&data)?;
                let twos = (!sum).wrapping_add(1);
                if options.little_endian_output {
                    twos.to_le_bytes().to_vec()
                } else {
                    twos.to_be_bytes().to_vec()
                }
            }
            ChecksumAlgorithm::WordSumLeTwosComplement => {
                let sum = word_sum_le(&data)?;
                let twos = (!sum).wrapping_add(1);
                if options.little_endian_output {
                    twos.to_le_bytes().to_vec()
                } else {
                    twos.to_be_bytes().to_vec()
                }
            }
            ChecksumAlgorithm::ModularSum => {
                let sum = byte_sum(&data);
                if options.little_endian_output {
                    sum.to_le_bytes().to_vec()
                } else {
                    sum.to_be_bytes().to_vec()
                }
            }
            ChecksumAlgorithm::Crc16 => {
                let crc = crc16_arc(&data);
                if options.little_endian_output {
                    crc.to_le_bytes().to_vec()
                } else {
                    crc.to_be_bytes().to_vec()
                }
            }
            ChecksumAlgorithm::Crc32 => {
                let crc = crc32_iso_hdlc(&data);
                if options.little_endian_output {
                    crc.to_le_bytes().to_vec()
                } else {
                    crc.to_be_bytes().to_vec()
                }
            }
            ChecksumAlgorithm::Crc16CcittLe => {
                let crc = crc16_ibm_sdlc(&data);
                crc.to_le_bytes().to_vec()
            }
            ChecksumAlgorithm::Crc16CcittBe => {
                let crc = crc16_ibm_sdlc(&data);
                crc.to_be_bytes().to_vec()
            }
            ChecksumAlgorithm::Crc16CcittLeInit0 => {
                let crc = crc16_xmodem(&data);
                crc.to_le_bytes().to_vec()
            }
            ChecksumAlgorithm::Crc16CcittBeInit0 => {
                let crc = crc16_xmodem(&data);
                crc.to_be_bytes().to_vec()
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
        let result = self.calculate_checksum(options)?;

        match target {
            ChecksumTarget::Address(addr) => {
                self.write_bytes(*addr, &result);
            }
            ChecksumTarget::Append => {
                if let Some(end) = self.max_address() {
                    self.write_bytes(end + 1, &result);
                }
            }
            ChecksumTarget::Prepend => {
                if let Some(start) = self.min_address() {
                    let new_start = start.saturating_sub(result.len() as u32);
                    self.write_bytes(new_start, &result);
                }
            }
            ChecksumTarget::OverwriteEnd => {
                if let Some(end) = self.max_address() {
                    // Write checksum to overwrite the last N bytes
                    // For N bytes ending at `end`, start address is `end - (N - 1)`
                    let offset = (result.len() as u32).saturating_sub(1);
                    let write_addr = end.saturating_sub(offset);
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
    fn collect_data_for_checksum(&self, range: Option<Range>) -> Result<Vec<u8>, OpsError> {
        let normalized = self.normalized_lossy();

        let mut filtered = normalized;
        if let Some(r) = range {
            filtered.filter_range(r);
        }

        // For checksums, we need contiguous data.
        // Fill gaps with 0xFF (typical flash default).
        filtered.fill_gaps(0xFF);

        if filtered.segments().is_empty() {
            return Ok(Vec::new());
        }

        Ok(filtered.segments()[0].data.clone())
    }
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
    Ok(data
        .chunks_exact(2)
        .fold(0u16, |acc, chunk| acc.wrapping_add(u16::from_be_bytes([chunk[0], chunk[1]]))))
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
    Ok(data
        .chunks_exact(2)
        .fold(0u16, |acc, chunk| acc.wrapping_add(u16::from_le_bytes([chunk[0], chunk[1]]))))
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
        };
        hf.checksum(&options, &ChecksumTarget::Append).unwrap();

        let norm = hf.normalized_lossy();
        assert_eq!(norm.max_address(), Some(0x1003));
    }

    #[test]
    fn test_hexfile_checksum_overwrite_end() {
        // Data at 0x1000-0x1003 (4 bytes), checksum is 2 bytes
        // OverwriteEnd should write at 0x1002-0x1003
        let mut hf = HexFile::with_segments(vec![Segment::new(0x1000, vec![0x01, 0x02, 0x03, 0x04])]);
        let options = ChecksumOptions {
            algorithm: ChecksumAlgorithm::ByteSumBe,
            range: None,
            little_endian_output: false,
        };
        hf.checksum(&options, &ChecksumTarget::OverwriteEnd).unwrap();

        let norm = hf.normalized_lossy();
        assert_eq!(norm.segments().len(), 1);
        assert_eq!(norm.min_address(), Some(0x1000));
        assert_eq!(norm.max_address(), Some(0x1003)); // Same end address
        // First two bytes unchanged, last two overwritten with checksum (0x000A)
        assert_eq!(norm.segments()[0].data, vec![0x01, 0x02, 0x00, 0x0A]);
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
        };
        hf.checksum(&options, &ChecksumTarget::OverwriteEnd).unwrap();

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
        };
        let result = hf.calculate_checksum(&options).unwrap();
        // Range extracts "123456789"
        assert_eq!(result, vec![0xBB, 0x3D]);
    }
}

use super::ParseError;
use crate::{HexFile, Segment};

const RECORD_DATA: u8 = 0x00;
const RECORD_EOF: u8 = 0x01;
const RECORD_EXTENDED_SEGMENT: u8 = 0x02;
const RECORD_EXTENDED_LINEAR: u8 = 0x04;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IntelHexMode {
    #[default]
    Auto,
    ExtendedLinear,
    ExtendedSegment,
}

#[derive(Debug, Clone)]
pub struct IntelHexWriteOptions {
    pub bytes_per_line: u8,
    pub mode: IntelHexMode,
}

impl Default for IntelHexWriteOptions {
    fn default() -> Self {
        Self {
            bytes_per_line: 16,
            mode: IntelHexMode::Auto,
        }
    }
}

pub fn parse_intel_hex(input: &[u8]) -> Result<HexFile, ParseError> {
    let text = std::str::from_utf8(input).map_err(|e| ParseError::InvalidRecord {
        line: 1,
        message: format!("invalid UTF-8: {e}"),
    })?;

    let mut segments: Vec<Segment> = Vec::new();
    let mut current_segment: Option<Segment> = None;
    let mut extended_address: u32 = 0;
    let mut eof_seen = false;

    for (line_num, line) in text.lines().enumerate() {
        let line_num = line_num + 1;
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        if eof_seen {
            return Err(ParseError::InvalidRecord {
                line: line_num,
                message: "data after EOF record".to_string(),
            });
        }

        if !line.starts_with(':') {
            return Err(ParseError::InvalidRecord {
                line: line_num,
                message: "line does not start with ':'".to_string(),
            });
        }

        let hex_str = &line[1..];
        if hex_str.len() < 10 {
            return Err(ParseError::InvalidRecord {
                line: line_num,
                message: "record too short".to_string(),
            });
        }

        let bytes = parse_hex_bytes(hex_str, line_num)?;
        validate_checksum(&bytes, line_num)?;

        let byte_count = bytes[0] as usize;
        let address = u16::from_be_bytes([bytes[1], bytes[2]]);
        let record_type = bytes[3];
        let data = &bytes[4..4 + byte_count];

        if bytes.len() != 5 + byte_count {
            return Err(ParseError::InvalidRecord {
                line: line_num,
                message: format!(
                    "byte count mismatch: header says {}, got {}",
                    byte_count,
                    bytes.len() - 5
                ),
            });
        }

        match record_type {
            RECORD_DATA => {
                let full_address = extended_address
                    .checked_add(address as u32)
                    .ok_or_else(|| ParseError::AddressOverflow(format!("line {line_num}")))?;

                match &mut current_segment {
                    Some(seg) if seg.end_address() + 1 == full_address => {
                        seg.data.extend_from_slice(data);
                    }
                    Some(seg) => {
                        segments.push(std::mem::replace(
                            seg,
                            Segment::new(full_address, data.to_vec()),
                        ));
                    }
                    None => {
                        current_segment = Some(Segment::new(full_address, data.to_vec()));
                    }
                }
            }
            RECORD_EOF => {
                eof_seen = true;
            }
            RECORD_EXTENDED_SEGMENT => {
                if byte_count != 2 {
                    return Err(ParseError::InvalidRecord {
                        line: line_num,
                        message: "extended segment address must have 2 data bytes".to_string(),
                    });
                }
                if let Some(seg) = current_segment.take() {
                    segments.push(seg);
                }
                let base = u16::from_be_bytes([data[0], data[1]]);
                extended_address = (base as u32) << 4;
            }
            RECORD_EXTENDED_LINEAR => {
                if byte_count != 2 {
                    return Err(ParseError::InvalidRecord {
                        line: line_num,
                        message: "extended linear address must have 2 data bytes".to_string(),
                    });
                }
                if let Some(seg) = current_segment.take() {
                    segments.push(seg);
                }
                let base = u16::from_be_bytes([data[0], data[1]]);
                extended_address = (base as u32) << 16;
            }
            0x03 | 0x05 => {}
            _ => {
                return Err(ParseError::UnsupportedRecordType {
                    line: line_num,
                    record_type,
                });
            }
        }
    }

    if !eof_seen {
        return Err(ParseError::UnexpectedEof);
    }

    if let Some(seg) = current_segment {
        segments.push(seg);
    }

    Ok(HexFile::with_segments(segments))
}

pub fn write_intel_hex(hexfile: &HexFile, options: &IntelHexWriteOptions) -> Vec<u8> {
    let normalized = hexfile.normalized_lossy();
    let mut output = Vec::new();
    let bytes_per_line = options.bytes_per_line.max(1) as usize;

    let mode = match options.mode {
        IntelHexMode::Auto => {
            if normalized.max_address().unwrap_or(0) > 0xFFFFF {
                IntelHexMode::ExtendedLinear
            } else if normalized.max_address().unwrap_or(0) > 0xFFFF {
                IntelHexMode::ExtendedSegment
            } else {
                IntelHexMode::ExtendedLinear
            }
        }
        m => m,
    };

    let mut current_extended: Option<u16> = None;

    for segment in normalized.segments() {
        let mut addr = segment.start_address;
        let mut data_offset = 0;

        while data_offset < segment.len() {
            let needed_extended = match mode {
                IntelHexMode::ExtendedLinear => (addr >> 16) as u16,
                IntelHexMode::ExtendedSegment => ((addr >> 4) & 0xF000) as u16,
                IntelHexMode::Auto => unreachable!(),
            };

            if current_extended != Some(needed_extended) {
                current_extended = Some(needed_extended);
                let record_type = match mode {
                    IntelHexMode::ExtendedLinear => RECORD_EXTENDED_LINEAR,
                    IntelHexMode::ExtendedSegment => RECORD_EXTENDED_SEGMENT,
                    IntelHexMode::Auto => unreachable!(),
                };
                write_record(&mut output, record_type, 0, &needed_extended.to_be_bytes());
            }

            let offset_addr = match mode {
                IntelHexMode::ExtendedLinear => (addr & 0xFFFF) as u16,
                IntelHexMode::ExtendedSegment => (addr & 0xFFFF) as u16,
                IntelHexMode::Auto => unreachable!(),
            };

            let remaining_in_bank = 0x10000u32.saturating_sub(offset_addr as u32) as usize;
            let remaining_data = segment.len() - data_offset;
            let chunk_len = bytes_per_line.min(remaining_in_bank).min(remaining_data);

            let chunk = &segment.data[data_offset..data_offset + chunk_len];
            write_record(&mut output, RECORD_DATA, offset_addr, chunk);

            data_offset += chunk_len;
            addr = addr.wrapping_add(chunk_len as u32);
        }
    }

    write_record(&mut output, RECORD_EOF, 0, &[]);
    output
}

fn write_record(output: &mut Vec<u8>, record_type: u8, address: u16, data: &[u8]) {
    let byte_count = data.len() as u8;
    let addr_bytes = address.to_be_bytes();

    let mut checksum: u8 = 0;
    checksum = checksum.wrapping_add(byte_count);
    checksum = checksum.wrapping_add(addr_bytes[0]);
    checksum = checksum.wrapping_add(addr_bytes[1]);
    checksum = checksum.wrapping_add(record_type);
    for &b in data {
        checksum = checksum.wrapping_add(b);
    }
    checksum = (!checksum).wrapping_add(1);

    output.push(b':');
    write_hex_byte(output, byte_count);
    write_hex_byte(output, addr_bytes[0]);
    write_hex_byte(output, addr_bytes[1]);
    write_hex_byte(output, record_type);
    for &b in data {
        write_hex_byte(output, b);
    }
    write_hex_byte(output, checksum);
    output.push(b'\n');
}

fn write_hex_byte(output: &mut Vec<u8>, byte: u8) {
    const HEX_CHARS: &[u8; 16] = b"0123456789ABCDEF";
    output.push(HEX_CHARS[(byte >> 4) as usize]);
    output.push(HEX_CHARS[(byte & 0x0F) as usize]);
}

fn parse_hex_bytes(hex_str: &str, line_num: usize) -> Result<Vec<u8>, ParseError> {
    if !hex_str.len().is_multiple_of(2) {
        return Err(ParseError::InvalidRecord {
            line: line_num,
            message: "odd number of hex digits".to_string(),
        });
    }

    let mut bytes = Vec::with_capacity(hex_str.len() / 2);
    let chars: Vec<char> = hex_str.chars().collect();

    for i in (0..chars.len()).step_by(2) {
        let high = hex_digit(chars[i], line_num)?;
        let low = hex_digit(chars[i + 1], line_num)?;
        bytes.push((high << 4) | low);
    }

    Ok(bytes)
}

fn hex_digit(c: char, line_num: usize) -> Result<u8, ParseError> {
    match c {
        '0'..='9' => Ok(c as u8 - b'0'),
        'A'..='F' => Ok(c as u8 - b'A' + 10),
        'a'..='f' => Ok(c as u8 - b'a' + 10),
        _ => Err(ParseError::InvalidHexDigit {
            line: line_num,
            char: c,
        }),
    }
}

fn validate_checksum(bytes: &[u8], line_num: usize) -> Result<(), ParseError> {
    let sum: u8 = bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    if sum != 0 {
        let actual = *bytes.last().unwrap();
        let expected = (!bytes[..bytes.len() - 1]
            .iter()
            .fold(0u8, |acc, &b| acc.wrapping_add(b)))
        .wrapping_add(1);
        return Err(ParseError::ChecksumMismatch {
            line: line_num,
            expected,
            actual,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let input = b":10010000214601360121470136007EFE09D2190140\n\
                      :100110002146017E17C20001FF5F16002148011928\n\
                      :00000001FF\n";
        let hf = parse_intel_hex(input).unwrap();
        assert_eq!(hf.segments().len(), 1);
        assert_eq!(hf.segments()[0].start_address, 0x0100);
        assert_eq!(hf.segments()[0].len(), 32);
    }

    #[test]
    fn test_parse_extended_linear() {
        let input = b":020000040800F2\n\
                      :10000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00\n\
                      :00000001FF\n";
        let hf = parse_intel_hex(input).unwrap();
        assert_eq!(hf.segments().len(), 1);
        assert_eq!(hf.segments()[0].start_address, 0x08000000);
    }

    #[test]
    fn test_parse_extended_segment() {
        let input = b":020000021000EC\n\
                      :10000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00\n\
                      :00000001FF\n";
        let hf = parse_intel_hex(input).unwrap();
        assert_eq!(hf.segments().len(), 1);
        assert_eq!(hf.segments()[0].start_address, 0x00010000);
    }

    #[test]
    fn test_checksum_error() {
        let input = b":10010000214601360121470136007EFE09D2190141\n\
                      :00000001FF\n";
        let result = parse_intel_hex(input);
        assert!(matches!(result, Err(ParseError::ChecksumMismatch { .. })));
    }

    #[test]
    fn test_missing_eof() {
        let input = b":10010000214601360121470136007EFE09D2190140\n";
        let result = parse_intel_hex(input);
        assert!(matches!(result, Err(ParseError::UnexpectedEof)));
    }

    #[test]
    fn test_roundtrip() {
        let input = b":020000040800F2\n\
                      :10000000000102030405060708090A0B0C0D0E0F78\n\
                      :10001000101112131415161718191A1B1C1D1E1F68\n\
                      :00000001FF\n";
        let hf = parse_intel_hex(input).unwrap();
        let output = write_intel_hex(&hf, &IntelHexWriteOptions::default());
        let hf2 = parse_intel_hex(&output).unwrap();
        assert_eq!(hf, hf2);
    }

    #[test]
    fn test_write_simple() {
        let hf = HexFile::with_segments(vec![Segment::new(0x0100, vec![0x00, 0x01, 0x02, 0x03])]);
        let output = write_intel_hex(&hf, &IntelHexWriteOptions::default());
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains(":020000040000FA"));
        assert!(text.contains(":0401000000010203F5"));
        assert!(text.contains(":00000001FF"));
    }
}

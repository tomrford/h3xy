mod binary;
mod c_code;
mod error;
mod hex_ascii;
mod intel_hex;
mod srec;

use crate::Segment;

pub use binary::{BinaryWriteOptions, parse_binary, write_binary};
pub use c_code::{CCodeOutput, CCodeWordType, CCodeWriteOptions, write_c_code};
pub use error::ParseError;
pub use hex_ascii::{HexAsciiWriteOptions, parse_hex_ascii, write_hex_ascii};
pub use intel_hex::{
    IntelHexMode, IntelHexWriteOptions, parse_intel_hex, parse_intel_hex_16bit, write_intel_hex,
};
pub use srec::{SRecordType, SRecordWriteOptions, parse_srec, write_srec};

fn normalized_sorted_segments(hexfile: &crate::HexFile) -> Vec<Segment> {
    let mut segments = hexfile.normalized_lossy().into_segments();
    segments.sort_by_key(|s| s.start_address);
    segments
}

pub mod error;
pub mod hexfile;
pub mod io;
pub mod ops;
pub mod range;
pub mod segment;

pub use error::Error;
pub use hexfile::{HexFile, HexFileError};
pub use io::{
    BinaryWriteOptions, CCodeOutput, CCodeWordType, CCodeWriteOptions, HexAsciiWriteOptions,
    SRecordType, SRecordWriteOptions, parse_binary, parse_hex_ascii, parse_srec, write_binary,
    write_c_code, write_hex_ascii, write_srec,
};
pub use io::{
    IntelHexMode, IntelHexWriteOptions, ParseError, parse_intel_hex, parse_intel_hex_16bit,
    write_intel_hex,
};
pub use ops::{
    AlignOptions, BankedMapOptions, ChecksumAlgorithm, ChecksumJob, ChecksumOptions,
    ChecksumTarget, FillOptions, ForcedRange, MergeMode, MergeOptions, OpsError, RemapOptions,
    SwapMode,
};
pub use range::{AddressRange, AddressRangeError, parse_hexview_ranges, parse_ranges};
pub use segment::Segment;

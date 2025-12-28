pub mod error;
pub mod hexfile;
pub mod io;
pub mod ops;
pub mod range;
pub mod segment;

pub use error::Error;
pub use hexfile::{HexFile, HexFileError};
pub use io::{IntelHexMode, IntelHexWriteOptions, ParseError, parse_intel_hex, write_intel_hex};
pub use ops::{AlignOptions, FillOptions, MergeMode, MergeOptions, OpsError, SwapMode};
pub use range::{Range, RangeError, parse_ranges};
pub use segment::Segment;

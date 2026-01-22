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
    AlignOptions, BankedMapOptions, ChecksumAlgorithm, ChecksumOptions, ChecksumTarget,
    FillOptions, ForcedRange, LogCommand, LogCommandKind, LogError, MergeMode, MergeOptions,
    OpsError, Pipeline, PipelineChecksum, PipelineError, PipelineMerge, PipelineResult,
    RemapOptions, SwapMode, execute_log_commands, execute_log_file, flag_align, flag_checksum,
    flag_cut_ranges, flag_execute_log_file, flag_fill_all, flag_fill_ranges_pattern,
    flag_fill_ranges_random, flag_filter_ranges, flag_map_star08, flag_map_star12,
    flag_map_star12x, flag_merge_opaque, flag_merge_transparent, flag_remap, flag_split,
    flag_swap_long, flag_swap_word, parse_log_commands, random_fill_bytes,
    random_fill_seed_from_time,
};
pub use range::{Range, RangeError, parse_hexview_ranges, parse_ranges};
pub use segment::Segment;

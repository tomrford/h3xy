//! HexView-compatible CLI argument parsing.
//!
//! Supports both `/` and `-` prefixes with `:` value separators.
//! Case-insensitive option matching.
//!
//! Processing order matches HexView exactly:
//! 1. Read input file
//! 2. Open error log (/E)
//! 3. Set silent mode (/S)
//! 4. Import 16-bit Hex (/II2)
//! 5. Address mapping (/s12map, /remap, etc.)
//! 6. Fill ranges (/FR)
//! 7. Cut ranges (/CR)
//! 8. Merge files (/MT, /MO)
//! 9. Address range filter (/AR)
//! 10. Execute log commands (/L)
//! 11. Create single-region (/FA)
//! 12. Postbuild (/PB)
//! 13. Align (/AD, /AL)
//! 14. Checksum (/CS)
//! 15. Data processing (/DP)
//! 16. Export (/Xx)

use std::path::PathBuf;
use std::process::ExitCode;

use h3xy::{FillOptions, HexFile, MergeMode, MergeOptions, Range};

#[derive(Debug, Default)]
pub struct Args {
    // Input
    pub input_file: Option<PathBuf>,

    // Output (special: uses space separator)
    pub output_file: Option<PathBuf>,

    // Error log: /E=file
    pub error_log: Option<PathBuf>,

    // Silent mode: /S
    pub silent: bool,

    // Import 16-bit Intel HEX: /II2=file
    pub import_i16: Option<PathBuf>,

    // Address mapping
    pub remap: Option<RemapParams>,
    pub s08_map: bool,
    pub s12_map: bool,
    pub s12x_map: bool,

    // Fill ranges: /FR:'range' with /FP:pattern
    pub fill_ranges: Vec<Range>,
    pub fill_pattern: Vec<u8>,

    // Cut ranges: /CR:'range1':'range2'
    pub cut_ranges: Vec<Range>,

    // Merge: /MO:file[;offset] or /MT:file[;offset]
    pub merge_opaque: Vec<MergeParam>,
    pub merge_transparent: Vec<MergeParam>,

    // Address range filter: /AR:'range'
    pub address_range: Vec<Range>,

    // Log file: /L:file
    pub log_file: Option<PathBuf>,

    // Single region: /FA
    pub fill_all: bool,

    // Postbuild: /PB:"file"
    pub postbuild: Option<PathBuf>,

    // Alignment: /AD:xx, /AL, /AF:xx
    pub align_address: Option<u32>,
    pub align_length: bool,
    pub align_fill: u8,
    pub align_erase: Option<u32>, // /AE:zzzz

    // Checksum: /CSx:target or /CSRx:target (little-endian)
    pub checksum: Option<ChecksumParams>,

    // Data processing: /DPn:param
    pub data_processing: Option<DataProcessingParams>,

    // Split blocks: /sb:size
    pub split_block_size: Option<u32>,

    // Byte swap: /swapword or /swaplong
    pub swap_word: bool,
    pub swap_long: bool,

    // dsPIC operations
    pub dspic_expand: Vec<DspicOp>,
    pub dspic_shrink: Vec<DspicOp>,
    pub dspic_clear_ghost: Vec<Range>,

    // Output format (only one allowed)
    pub output_format: Option<OutputFormat>,

    // Output format options
    pub bytes_per_line: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct MergeParam {
    pub file: PathBuf,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct RemapParams {
    pub start: u32,
    pub end: u32,
    pub linear: u32,
    pub size: u32,
    pub inc: u32,
}

#[derive(Debug, Clone)]
pub struct ChecksumParams {
    pub algorithm: u8,
    pub target: ChecksumTarget,
    pub little_endian: bool,
    pub range: Option<Range>,
}

#[derive(Debug, Clone)]
pub enum ChecksumTarget {
    Address(u32),
    File(PathBuf),
}

#[derive(Debug, Clone)]
pub struct DataProcessingParams {
    pub method: u8,
    pub param: String,
}

#[derive(Debug, Clone)]
pub struct DspicOp {
    pub range: Range,
    pub target: Option<u32>,
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
    IntelHex { record_type: Option<u8> }, // /XI[:len[:type]]
    SRecord { record_type: Option<u8> },  // /XS[:len[:type]]
    Binary,                               // /XN
    HexAscii { len: Option<u8>, sep: Option<char> }, // /XA
    CCode,                                // /XC
    FordIntelHex,                         // /XF
    GmHeader { addr: Option<u32> },       // /XG
    GmHeaderOs { addr: Option<u32> },     // /XGC
    GmHeaderCal { addr: Option<u32> },    // /XGCC
    Gac,                                  // /XGAC
    GacSwil,                              // /XGACSWIL
    FlashKernel,                          // /XK
    Porsche,                              // /XP
    SeparateBinary,                       // /XSB
    Vag,                                  // /XV
    Vbf,                                  // /XVBF
    FiatBin,                              // /XB
}

#[derive(Debug)]
pub enum ParseArgError {
    MissingInputFile,
    MissingOutputFile,
    InvalidOption(String),
    InvalidRange(String),
    InvalidNumber(String),
    DuplicateOutputFormat,
    MissingValue(String),
}

impl std::fmt::Display for ParseArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingInputFile => write!(f, "missing input file"),
            Self::MissingOutputFile => write!(f, "missing output file (-o)"),
            Self::InvalidOption(s) => write!(f, "invalid option: {s}"),
            Self::InvalidRange(s) => write!(f, "invalid range: {s}"),
            Self::InvalidNumber(s) => write!(f, "invalid number: {s}"),
            Self::DuplicateOutputFormat => write!(f, "multiple output formats specified"),
            Self::MissingValue(s) => write!(f, "missing value for {s}"),
        }
    }
}

impl std::error::Error for ParseArgError {}

impl Args {
    pub fn parse() -> Result<Self, ParseArgError> {
        Self::parse_from(std::env::args().skip(1).collect())
    }

    pub fn parse_from(args: Vec<String>) -> Result<Self, ParseArgError> {
        let mut result = Args::default();
        result.fill_pattern = vec![0xFF];
        result.align_fill = 0xFF;

        let mut args_iter = args.iter().peekable();

        while let Some(arg) = args_iter.next() {
            // Handle -o specially (space-separated)
            if arg.eq_ignore_ascii_case("-o") {
                let next = args_iter
                    .next()
                    .ok_or(ParseArgError::MissingValue("-o".into()))?;
                result.output_file = Some(PathBuf::from(next));
                continue;
            }

            // Check for option prefix
            if let Some(opt) = arg.strip_prefix('/').or_else(|| arg.strip_prefix('-')) {
                parse_option(&mut result, opt)?;
            } else if result.input_file.is_none() {
                result.input_file = Some(PathBuf::from(arg));
            } else {
                return Err(ParseArgError::InvalidOption(arg.clone()));
            }
        }

        Ok(result)
    }

    /// Execute the parsed arguments in HexView processing order.
    pub fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Read input file
        let mut hexfile = if let Some(ref path) = self.input_file {
            load_input(path)?
        } else {
            return Err(ParseArgError::MissingInputFile.into());
        };

        // 2. Open error log (/E)
        if let Some(ref _path) = self.error_log {
            // TODO: set up error logging
        }

        // 3. Silent mode (/S) - already handled by flag

        // 4. Import 16-bit Hex (/II2)
        if let Some(ref _path) = self.import_i16 {
            // TODO: import and merge 16-bit Intel HEX
        }

        // 5. Address mapping (/s12map, /remap, etc.)
        if self.s08_map {
            // TODO: apply S08 address mapping
        }
        if self.s12_map {
            // TODO: apply S12 address mapping
        }
        if self.s12x_map {
            // TODO: apply S12X address mapping
        }
        if let Some(ref _remap) = self.remap {
            // TODO: apply manual remap
        }

        // 6. Fill ranges (/FR)
        for range in &self.fill_ranges {
            let options = FillOptions {
                pattern: self.fill_pattern.clone(),
                overwrite: false,
            };
            hexfile.fill(*range, &options);
        }

        // 7. Cut ranges (/CR)
        for range in &self.cut_ranges {
            hexfile.cut(*range);
        }

        // 8. Merge files (/MT, /MO)
        for merge in &self.merge_transparent {
            let other = load_input(&merge.file)?;
            let options = MergeOptions {
                mode: MergeMode::Preserve,
                offset: merge.offset.unwrap_or(0),
                range: None,
            };
            hexfile.merge(&other, &options);
        }
        for merge in &self.merge_opaque {
            let other = load_input(&merge.file)?;
            let options = MergeOptions {
                mode: MergeMode::Overwrite,
                offset: merge.offset.unwrap_or(0),
                range: None,
            };
            hexfile.merge(&other, &options);
        }

        // 9. Address range filter (/AR)
        if !self.address_range.is_empty() {
            hexfile.filter_ranges(&self.address_range);
        }

        // 10. Execute log commands (/L)
        if let Some(ref _path) = self.log_file {
            // TODO: execute HexView log/macro file
        }

        // 11. Create single-region (/FA)
        if self.fill_all {
            hexfile.fill_gaps(self.align_fill);
        }

        // 12. Postbuild (/PB)
        if let Some(ref _path) = self.postbuild {
            // TODO: run postbuild operations
        }

        // 13. Align (/AD, /AL)
        if let Some(alignment) = self.align_address {
            let options = h3xy::AlignOptions {
                alignment,
                fill_byte: self.align_fill,
                align_length: self.align_length,
            };
            hexfile.align(&options)?;
        }

        // dsPIC operations (order TBD based on HexView docs)
        for op in &self.dspic_expand {
            // TODO: expand dsPIC data
            let _ = op;
        }
        for op in &self.dspic_shrink {
            // TODO: shrink dsPIC data
            let _ = op;
        }
        for range in &self.dspic_clear_ghost {
            // TODO: clear ghost bytes
            let _ = range;
        }

        // Split blocks
        if let Some(size) = self.split_block_size {
            hexfile.split(size);
        }

        // Byte swap
        if self.swap_word {
            hexfile.swap_bytes(h3xy::SwapMode::Word)?;
        }
        if self.swap_long {
            hexfile.swap_bytes(h3xy::SwapMode::DWord)?;
        }

        // 14. Checksum (/CS)
        if let Some(ref _cs) = self.checksum {
            // TODO: calculate and insert checksum
        }

        // 15. Data processing (/DP)
        if let Some(ref _dp) = self.data_processing {
            // TODO: run external data processing
        }

        // 16. Export (/Xx)
        if let Some(ref path) = self.output_file {
            write_output(&hexfile, path, &self.output_format, self.bytes_per_line)?;
        }

        Ok(())
    }
}

fn parse_option(args: &mut Args, opt: &str) -> Result<(), ParseArgError> {
    let opt_upper = opt.to_ascii_uppercase();

    // Silent mode
    if opt_upper == "S" {
        args.silent = true;
        return Ok(());
    }

    // Fill all gaps
    if opt_upper == "FA" {
        args.fill_all = true;
        return Ok(());
    }

    // Swap operations
    if opt_upper == "SWAPWORD" {
        args.swap_word = true;
        return Ok(());
    }
    if opt_upper == "SWAPLONG" {
        args.swap_long = true;
        return Ok(());
    }

    // Preset address mappings
    if opt_upper == "S08" {
        args.s08_map = true;
        return Ok(());
    }
    if opt_upper == "S12MAP" {
        args.s12_map = true;
        return Ok(());
    }
    if opt_upper == "S12XMAP" {
        args.s12x_map = true;
        return Ok(());
    }

    // Options with values (colon or equals separated)
    if let Some((key, value)) = split_option(opt) {
        let key_upper = key.to_ascii_uppercase();

        match key_upper.as_str() {
            // Error log: /E=file
            "E" => {
                args.error_log = Some(PathBuf::from(strip_quotes(value)));
            }

            // Import 16-bit Intel HEX: /II2=file
            "II2" => {
                args.import_i16 = Some(PathBuf::from(strip_quotes(value)));
            }

            // Address range filter: /AR:'range'
            "AR" => {
                let ranges = parse_hexview_ranges(value)?;
                args.address_range.extend(ranges);
            }

            // Cut ranges: /CR:'range1':'range2'
            "CR" => {
                let ranges = parse_hexview_ranges(value)?;
                args.cut_ranges.extend(ranges);
            }

            // Fill ranges: /FR:'range'
            "FR" => {
                let ranges = parse_hexview_ranges(value)?;
                args.fill_ranges.extend(ranges);
            }

            // Fill pattern: /FP:xxyyzz
            "FP" => {
                args.fill_pattern = parse_hex_bytes(value)?;
            }

            // Merge opaque: /MO:file[;offset]
            "MO" => {
                args.merge_opaque.push(parse_merge_param(value)?);
            }

            // Merge transparent: /MT:file[;offset]
            "MT" => {
                args.merge_transparent.push(parse_merge_param(value)?);
            }

            // Log file: /L:file
            "L" => {
                args.log_file = Some(PathBuf::from(strip_quotes(value)));
            }

            // Postbuild: /PB:"file"
            "PB" => {
                args.postbuild = Some(PathBuf::from(strip_quotes(value)));
            }

            // Align address: /AD:xx
            "AD" => {
                args.align_address = Some(parse_number(value)?);
            }

            // Align length: /AL[:length] (length optional)
            "AL" => {
                args.align_length = true;
                if !value.is_empty() {
                    args.align_address = Some(parse_number(value)?);
                }
            }

            // Align fill: /AF:xx
            "AF" => {
                args.align_fill = parse_number(value)? as u8;
            }

            // Align erase: /AE:zzzz
            "AE" => {
                args.align_erase = Some(parse_number(value)?);
            }

            // Split blocks: /sb:size
            "SB" => {
                args.split_block_size = Some(parse_number(value)?);
            }

            // Remap: /remap:Start-End,Linear,Size,Inc
            "REMAP" => {
                args.remap = Some(parse_remap(value)?);
            }

            // Checksum: /CSx:target or /CSRx:target
            s if s.starts_with("CSR") => {
                let algo = s.strip_prefix("CSR").unwrap();
                args.checksum = Some(parse_checksum(algo, value, true)?);
            }
            s if s.starts_with("CS") => {
                let algo = s.strip_prefix("CS").unwrap();
                args.checksum = Some(parse_checksum(algo, value, false)?);
            }

            // Data processing: /DPn:param
            s if s.starts_with("DP") => {
                let method_str = s.strip_prefix("DP").unwrap();
                let method = method_str
                    .parse::<u8>()
                    .map_err(|_| ParseArgError::InvalidNumber(method_str.to_string()))?;
                args.data_processing = Some(DataProcessingParams {
                    method,
                    param: value.to_string(),
                });
            }

            // dsPIC expand: /cdspx:range[;target]
            "CDSPX" => {
                args.dspic_expand.push(parse_dspic_op(value)?);
            }

            // dsPIC shrink: /cdsps:range[;target]
            "CDSPS" => {
                args.dspic_shrink.push(parse_dspic_op(value)?);
            }

            // dsPIC clear ghost: /cdspg:range
            "CDSPG" => {
                let ranges = parse_hexview_ranges(value)?;
                args.dspic_clear_ghost.extend(ranges);
            }

            // Output formats
            "XI" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                let (len, rec_type) = parse_output_params(value);
                args.bytes_per_line = len;
                args.output_format = Some(OutputFormat::IntelHex {
                    record_type: rec_type,
                });
            }

            "XS" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                let (len, rec_type) = parse_output_params(value);
                args.bytes_per_line = len;
                args.output_format = Some(OutputFormat::SRecord {
                    record_type: rec_type,
                });
            }

            "XN" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::Binary);
            }

            "XA" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                // TODO: parse len and sep
                args.output_format = Some(OutputFormat::HexAscii {
                    len: None,
                    sep: None,
                });
            }

            "XC" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::CCode);
            }

            "XF" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::FordIntelHex);
            }

            "XG" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                let addr = if value.is_empty() {
                    None
                } else {
                    Some(parse_number(value)?)
                };
                args.output_format = Some(OutputFormat::GmHeader { addr });
            }

            "XGC" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                let addr = if value.is_empty() {
                    None
                } else {
                    Some(parse_number(value)?)
                };
                args.output_format = Some(OutputFormat::GmHeaderOs { addr });
            }

            "XGCC" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                let addr = if value.is_empty() {
                    None
                } else {
                    Some(parse_number(value)?)
                };
                args.output_format = Some(OutputFormat::GmHeaderCal { addr });
            }

            "XGAC" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::Gac);
            }

            "XGACSWIL" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::GacSwil);
            }

            "XK" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::FlashKernel);
            }

            "XP" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::Porsche);
            }

            "XSB" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::SeparateBinary);
            }

            "XV" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::Vag);
            }

            "XVBF" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::Vbf);
            }

            "XB" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::FiatBin);
            }

            _ => return Err(ParseArgError::InvalidOption(opt.to_string())),
        }
    } else {
        // Handle bare output format options (no value)
        let opt_upper = opt.to_ascii_uppercase();
        match opt_upper.as_str() {
            "XI" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::IntelHex { record_type: None });
            }
            "XS" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::SRecord { record_type: None });
            }
            "XN" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::Binary);
            }
            "XA" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::HexAscii {
                    len: None,
                    sep: None,
                });
            }
            "XC" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::CCode);
            }
            "XF" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::FordIntelHex);
            }
            "XG" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::GmHeader { addr: None });
            }
            "XGC" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::GmHeaderOs { addr: None });
            }
            "XGCC" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::GmHeaderCal { addr: None });
            }
            "XGAC" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::Gac);
            }
            "XGACSWIL" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::GacSwil);
            }
            "XK" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::FlashKernel);
            }
            "XP" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::Porsche);
            }
            "XSB" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::SeparateBinary);
            }
            "XV" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::Vag);
            }
            "XVBF" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::Vbf);
            }
            "XB" => {
                if args.output_format.is_some() {
                    return Err(ParseArgError::DuplicateOutputFormat);
                }
                args.output_format = Some(OutputFormat::FiatBin);
            }
            "AL" => {
                args.align_length = true;
            }
            _ => return Err(ParseArgError::InvalidOption(opt.to_string())),
        }
    }

    Ok(())
}

/// Split option into key and value (supports : and = separators).
fn split_option(opt: &str) -> Option<(&str, &str)> {
    if let Some(pos) = opt.find(':') {
        Some((&opt[..pos], &opt[pos + 1..]))
    } else if let Some(pos) = opt.find('=') {
        Some((&opt[..pos], &opt[pos + 1..]))
    } else {
        None
    }
}

/// Strip surrounding quotes from a value.
fn strip_quotes(s: &str) -> &str {
    s.trim_matches(|c| c == '"' || c == '\'')
}

/// Parse HexView-style ranges (quoted, colon-separated).
fn parse_hexview_ranges(s: &str) -> Result<Vec<Range>, ParseArgError> {
    let s = strip_quotes(s);
    h3xy::parse_ranges(s).map_err(|e| ParseArgError::InvalidRange(e.to_string()))
}

/// Parse a hex byte string (e.g., "DEADBEEF" -> [0xDE, 0xAD, 0xBE, 0xEF]).
fn parse_hex_bytes(s: &str) -> Result<Vec<u8>, ParseArgError> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) {
        return Err(ParseArgError::InvalidNumber(format!(
            "odd-length hex string: {s}"
        )));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|_| ParseArgError::InvalidNumber(s[i..i + 2].to_string()))
        })
        .collect()
}

/// Parse a number (decimal, 0x hex, 0b binary).
fn parse_number(s: &str) -> Result<u32, ParseArgError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ParseArgError::InvalidNumber("empty".to_string()));
    }

    let (radix, digits) = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        (16, hex)
    } else if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        (2, bin)
    } else if let Some(bin) = s.strip_suffix('b').or_else(|| s.strip_suffix('B')) {
        (2, bin)
    } else if s.chars().all(|c| c.is_ascii_hexdigit()) && s.chars().any(|c| c.is_ascii_alphabetic())
    {
        // Pure hex without prefix (HexView style)
        (16, s)
    } else {
        (10, s)
    };

    u32::from_str_radix(digits, radix).map_err(|e| ParseArgError::InvalidNumber(e.to_string()))
}

/// Parse merge parameter: file[;offset].
fn parse_merge_param(s: &str) -> Result<MergeParam, ParseArgError> {
    let s = strip_quotes(s);
    if let Some((file, offset_str)) = s.split_once(';') {
        let offset = parse_number(offset_str)? as i64;
        Ok(MergeParam {
            file: PathBuf::from(file),
            offset: Some(offset),
        })
    } else {
        Ok(MergeParam {
            file: PathBuf::from(s),
            offset: None,
        })
    }
}

/// Parse remap parameters: Start-End,Linear,Size,Inc.
fn parse_remap(s: &str) -> Result<RemapParams, ParseArgError> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 4 {
        return Err(ParseArgError::InvalidOption(format!(
            "remap requires 4 parameters: {s}"
        )));
    }

    let (start_str, end_str) = parts[0].split_once('-').ok_or_else(|| {
        ParseArgError::InvalidOption(format!("remap range invalid: {}", parts[0]))
    })?;

    Ok(RemapParams {
        start: parse_number(start_str)?,
        end: parse_number(end_str)?,
        linear: parse_number(parts[1])?,
        size: parse_number(parts[2])?,
        inc: parse_number(parts[3])?,
    })
}

/// Parse checksum parameters.
fn parse_checksum(
    algo: &str,
    target: &str,
    little_endian: bool,
) -> Result<ChecksumParams, ParseArgError> {
    let algorithm = if algo.is_empty() {
        0
    } else {
        algo.parse::<u8>()
            .map_err(|_| ParseArgError::InvalidNumber(algo.to_string()))?
    };

    let target = if target.starts_with('@') {
        let addr = parse_number(&target[1..])?;
        ChecksumTarget::Address(addr)
    } else {
        ChecksumTarget::File(PathBuf::from(target))
    };

    Ok(ChecksumParams {
        algorithm,
        target,
        little_endian,
        range: None,
    })
}

/// Parse dsPIC operation: range[;target].
fn parse_dspic_op(s: &str) -> Result<DspicOp, ParseArgError> {
    let s = strip_quotes(s);
    if let Some((range_str, target_str)) = s.split_once(';') {
        let ranges = parse_hexview_ranges(range_str)?;
        let target = parse_number(target_str)?;
        Ok(DspicOp {
            range: ranges
                .into_iter()
                .next()
                .ok_or_else(|| ParseArgError::InvalidRange(s.to_string()))?,
            target: Some(target),
        })
    } else {
        let ranges = parse_hexview_ranges(s)?;
        Ok(DspicOp {
            range: ranges
                .into_iter()
                .next()
                .ok_or_else(|| ParseArgError::InvalidRange(s.to_string()))?,
            target: None,
        })
    }
}

/// Parse output format parameters: len[:type].
fn parse_output_params(s: &str) -> (Option<u8>, Option<u8>) {
    if s.is_empty() {
        return (None, None);
    }

    let parts: Vec<&str> = s.split(':').collect();
    let len = parts.first().and_then(|p| p.parse::<u8>().ok());
    let rec_type = parts.get(1).and_then(|p| p.parse::<u8>().ok());
    (len, rec_type)
}

/// Load input file (auto-detect format).
fn load_input(path: &PathBuf) -> Result<HexFile, Box<dyn std::error::Error>> {
    let content = std::fs::read(path)?;

    // Auto-detect format
    let first_line = content
        .iter()
        .take_while(|&&b| b != b'\n' && b != b'\r')
        .copied()
        .collect::<Vec<u8>>();

    if first_line.first() == Some(&b':') {
        // Intel HEX
        let hexfile = h3xy::parse_intel_hex(&content)?;
        Ok(hexfile)
    } else if first_line.first() == Some(&b'S') {
        // S-Record
        // TODO: implement S-Record parsing
        Err("S-Record parsing not yet implemented".into())
    } else {
        // Binary
        // TODO: implement binary loading with base address
        Err("Binary loading not yet implemented".into())
    }
}

/// Write output file.
fn write_output(
    hexfile: &HexFile,
    path: &PathBuf,
    format: &Option<OutputFormat>,
    bytes_per_line: Option<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = format
        .as_ref()
        .unwrap_or(&OutputFormat::IntelHex { record_type: None });

    match format {
        OutputFormat::IntelHex { record_type } => {
            let mode = match record_type {
                Some(1) => h3xy::IntelHexMode::ExtendedLinear,
                Some(2) => h3xy::IntelHexMode::ExtendedSegment,
                _ => h3xy::IntelHexMode::Auto,
            };
            let options = h3xy::IntelHexWriteOptions {
                bytes_per_line: bytes_per_line.unwrap_or(16),
                mode,
            };
            let output = h3xy::write_intel_hex(hexfile, &options);
            std::fs::write(path, output)?;
        }
        OutputFormat::SRecord { record_type: _ } => {
            // TODO: implement S-Record output
            return Err("S-Record output not yet implemented".into());
        }
        OutputFormat::Binary => {
            // TODO: implement binary output
            return Err("Binary output not yet implemented".into());
        }
        _ => {
            // TODO: implement other formats
            return Err(format!("Output format {:?} not yet implemented", format).into());
        }
    }

    Ok(())
}

pub fn run() -> ExitCode {
    let args = match Args::parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = args.execute() {
        if !args.silent {
            eprintln!("Error: {e}");
        }
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

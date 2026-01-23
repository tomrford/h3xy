use h3xy::{
    AlignOptions, ChecksumAlgorithm, Pipeline, PipelineDspic, PipelineError, PipelineMerge, Range,
    RemapOptions,
};

use super::error::{CliError, ExecuteOutput};
use super::io::{load_input, write_output_for_args};
use super::types::{Args, ChecksumTarget, ParseArgError};

impl Args {
    fn wrap_error<T, E: std::fmt::Display>(
        &self,
        opt: &str,
        res: Result<T, E>,
    ) -> Result<T, CliError> {
        res.map_err(|e| CliError::Other(format!("{opt}: {e}")))
    }

    fn validate_supported_features(&self) -> Result<(), CliError> {
        if !self.merge_transparent.is_empty() && !self.merge_opaque.is_empty() {
            return Err(CliError::Unsupported(
                "cannot combine /MT and /MO in one command".into(),
            ));
        }
        if self.s12_map && self.s12x_map {
            return Err(CliError::Unsupported(
                "cannot combine /S12MAP and /S12XMAP".into(),
            ));
        }
        if self.s08_map && (self.s12_map || self.s12x_map) {
            return Err(CliError::Unsupported(
                "cannot combine /S08MAP with /S12MAP or /S12XMAP".into(),
            ));
        }
        if self.remap.is_some() && (self.s12_map || self.s12x_map || self.s08_map) {
            return Err(CliError::Unsupported(
                "cannot combine /REMAP with /S12MAP or /S12XMAP".into(),
            ));
        }
        if self.postbuild.is_some() {
            return Err(CliError::Unsupported(
                "postbuild (/PB) is not supported yet".into(),
            ));
        }
        if self.data_processing.is_some() {
            return Err(CliError::Unsupported(
                "data processing (/DP) is not supported yet".into(),
            ));
        }
        if self.import_binary.is_some() && self.import_hex_ascii.is_some() {
            return Err(CliError::Unsupported(
                "binary import (/IN) cannot be combined with HEX ASCII import (/IA)".into(),
            ));
        }
        if self.import_i16.is_some()
            && (self.import_binary.is_some() || self.import_hex_ascii.is_some())
        {
            return Err(CliError::Unsupported(
                "16-bit Intel HEX import (/II2) cannot be combined with /IN or /IA".into(),
            ));
        }
        if (self.import_binary.is_some() || self.import_i16.is_some()) && self.input_file.is_some()
        {
            return Err(CliError::Unsupported(
                "explicit import (/IN, /II2) cannot be combined with input file".into(),
            ));
        }
        Ok(())
    }

    /// Execute the parsed arguments in HexView processing order.
    pub fn execute(&self) -> Result<ExecuteOutput, CliError> {
        self.validate_supported_features()?;

        let hexfile = self.load_hexfile()?;
        let pipeline = self.build_pipeline(hexfile)?;
        let result = pipeline
            .execute(random_fill_bytes, load_input)
            .map_err(|e| match e {
                PipelineError::Ops(err) => CliError::Other(err.to_string()),
                PipelineError::Log(err) => CliError::Other(format!("/L: {err}")),
            })?;
        let mut hexfile = result.hexfile;
        let checksum_bytes = self.apply_checksum(&mut hexfile)?;
        self.write_outputs(&hexfile)?;

        Ok(ExecuteOutput { checksum_bytes })
    }

    fn build_pipeline(&self, hexfile: h3xy::HexFile) -> Result<Pipeline, CliError> {
        let log_commands = if let Some(ref path) = self.log_file {
            let content = std::fs::read_to_string(path)
                .map_err(|e| CliError::Other(format!("/L: {e}")))?;
            Some(
                h3xy::parse_log_commands(&content)
                    .map_err(|e| CliError::Other(format!("/L: {e}")))?,
            )
        } else {
            None
        };

        let mut merge_transparent = Vec::with_capacity(self.merge_transparent.len());
        for merge in &self.merge_transparent {
            let other = load_input(&merge.file)?;
            merge_transparent.push(PipelineMerge {
                other,
                offset: merge.offset.unwrap_or(0),
                range: merge.range,
            });
        }
        let mut merge_opaque = Vec::with_capacity(self.merge_opaque.len());
        for merge in &self.merge_opaque {
            let other = load_input(&merge.file)?;
            merge_opaque.push(PipelineMerge {
                other,
                offset: merge.offset.unwrap_or(0),
                range: merge.range,
            });
        }

        let align = self.align_address.map(|alignment| AlignOptions {
            alignment,
            fill_byte: self.align_fill,
            align_length: self.align_length,
        });

        let mut pipeline = Pipeline::default();
        pipeline.hexfile = hexfile;
        pipeline.fill_ranges = self.fill_ranges.clone();
        pipeline.fill_pattern = if self.fill_pattern_set {
            Some(self.fill_pattern.clone())
        } else {
            None
        };
        pipeline.cut_ranges = self.cut_ranges.clone();
        pipeline.merge_transparent = merge_transparent;
        pipeline.merge_opaque = merge_opaque;
        pipeline.address_ranges = self.address_range.clone();
        pipeline.log_commands = log_commands;
        pipeline.fill_all = if self.fill_all { Some(self.align_fill) } else { None };
        pipeline.align = align;
        pipeline.split = self.split_block_size;
        pipeline.swap_word = self.swap_word;
        pipeline.swap_long = self.swap_long;
        pipeline.checksum = None;
        pipeline.map_star12 = self.s12_map;
        pipeline.map_star12x = self.s12x_map;
        pipeline.map_star08 = self.s08_map;
        pipeline.remap = self.remap.as_ref().map(|remap| RemapOptions {
            start: remap.start,
            end: remap.end,
            linear: remap.linear,
            size: remap.size,
            inc: remap.inc,
        });
        pipeline.dspic_expand = self
            .dspic_expand
            .iter()
            .map(|op| PipelineDspic {
                range: op.range,
                target: op.target,
            })
            .collect();
        pipeline.dspic_shrink = self
            .dspic_shrink
            .iter()
            .map(|op| PipelineDspic {
                range: op.range,
                target: op.target,
            })
            .collect();
        pipeline.dspic_clear_ghost = self.dspic_clear_ghost.clone();

        Ok(pipeline)
    }

    fn load_hexfile(&self) -> Result<h3xy::HexFile, CliError> {
        if let Some(ref import) = self.import_binary {
            return super::io::load_binary_input(&import.file, import.offset);
        }
        if let Some(ref import) = self.import_hex_ascii {
            let ascii = super::io::load_hex_ascii_input(&import.file, import.offset)?;
            if let Some(ref path) = self.input_file {
                let mut base = load_input(path)?;
                if super::io::hexfiles_overlap(&base, &ascii) {
                    if !self.silent {
                        eprintln!("Warning: /IA overlaps input file; ignoring input file");
                    }
                    return Ok(ascii);
                }
                for segment in ascii.segments() {
                    base.append_segment(segment.clone());
                }
                return Ok(base);
            }
            return Ok(ascii);
        }
        if let Some(ref import) = self.import_i16 {
            return super::io::load_intel_hex_16bit_input(import);
        }
        if let Some(ref path) = self.input_file {
            return load_input(path);
        }
        if self.log_file.is_some() {
            return Ok(h3xy::HexFile::new());
        }
        Err(ParseArgError::MissingInputFile.into())
    }

    fn apply_checksum(&self, hexfile: &mut h3xy::HexFile) -> Result<Option<Vec<u8>>, CliError> {
        let Some(ref cs_params) = self.checksum else {
            return Ok(None);
        };
        let opt_base = if cs_params.little_endian {
            "/CSR"
        } else {
            "/CS"
        };
        let opt = format!("{opt_base}{}", cs_params.algorithm);
        let algorithm =
            self.wrap_error(&opt, ChecksumAlgorithm::from_index(cs_params.algorithm))?;
        let forced_range = cs_params
            .forced_range
            .as_ref()
            .map(|forced| h3xy::ForcedRange {
                range: forced.range,
                pattern: forced.pattern.clone(),
            });
        let lib_target = match &cs_params.target {
            ChecksumTarget::Address(addr) => h3xy::ChecksumTarget::Address(*addr),
            ChecksumTarget::Append => h3xy::ChecksumTarget::Append,
            ChecksumTarget::Begin => {
                if let Some(start) = hexfile.min_address() {
                    h3xy::ChecksumTarget::Address(start)
                } else {
                    h3xy::ChecksumTarget::Append
                }
            }
            ChecksumTarget::Prepend => h3xy::ChecksumTarget::Prepend,
            ChecksumTarget::OverwriteEnd => h3xy::ChecksumTarget::OverwriteEnd,
            ChecksumTarget::File(path) => h3xy::ChecksumTarget::File(path.clone()),
        };
        let result = self.wrap_error(
            &opt,
            h3xy::flag_checksum(
                hexfile,
                algorithm,
                cs_params.range,
                cs_params.little_endian,
                forced_range,
                &cs_params.exclude_ranges,
                &lib_target,
            ),
        )?;

        if let ChecksumTarget::File(ref path) = cs_params.target {
            let formatted = result
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(",");
            self.wrap_error(&opt, std::fs::write(path, formatted))?;
        }

        Ok(Some(result))
    }

    fn write_outputs(&self, hexfile: &h3xy::HexFile) -> Result<(), CliError> {
        write_output_for_args(self, hexfile)
    }
}

fn random_fill_bytes(range: Range) -> Vec<u8> {
    let seed = h3xy::random_fill_seed_from_time(range);
    h3xy::random_fill_bytes(range, seed)
}

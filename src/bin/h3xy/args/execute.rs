use crate::{
    AlignOptions, ChecksumAlgorithm, Pipeline, PipelineDspic, PipelineError, PipelineMerge, Range,
    RemapOptions,
};

use super::error::{CliError, ExecuteOutput};
use super::io::{FsProvider, ReadProvider, write_output_for_args};
use super::io::{load_binary_input, load_hex_ascii_input, load_input, load_intel_hex_16bit_input};
use super::signature::{
    apply_data_processing, apply_signature_verification, is_supported_data_processing_method,
    is_supported_signature_verify_method,
};
use super::types::{Args, ChecksumTarget, ParseArgError};
use std::collections::HashMap;
use std::path::Path;

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
        if let Some(ref params) = self.data_processing
            && !is_supported_data_processing_method(params.method)
        {
            return Err(CliError::Unsupported(format!(
                "data processing (/DP{}) is not supported yet",
                params.method
            )));
        }
        if let Some(ref params) = self.signature_verify
            && !is_supported_signature_verify_method(params.method)
        {
            return Err(CliError::Unsupported(format!(
                "signature verification (/SV{}) is not supported yet",
                params.method
            )));
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
        let provider = FsProvider;
        self.execute_with_provider(&provider)
    }

    pub(super) fn execute_with_provider<P: ReadProvider>(
        &self,
        provider: &P,
    ) -> Result<ExecuteOutput, CliError> {
        self.validate_supported_features()?;

        let hexfile = self.load_hexfile(provider)?;
        let pipeline = self.build_pipeline(hexfile, provider)?;
        let result = pipeline
            .execute(random_fill_bytes, |path| load_input(provider, path))
            .map_err(|e| match e {
                PipelineError::Ops(err) => CliError::Other(err.to_string()),
                PipelineError::Log(err) => CliError::Other(format!("/L: {err}")),
        })?;
        let mut hexfile = result.hexfile;
        let checksum_bytes = self.apply_checksum(&mut hexfile)?;
        let _signature_bytes = self.apply_data_processing(&mut hexfile)?;
        self.apply_signature_verification(&hexfile)?;
        self.write_outputs(&hexfile, provider)?;

        Ok(ExecuteOutput { checksum_bytes })
    }

    pub(super) fn execute_with_blocks(
        &self,
        blocks: &HashMap<String, crate::HexFile>,
    ) -> Result<ExecuteOutput, CliError> {
        self.validate_supported_features()?;

        let provider = FsProvider;
        let hexfile = self.load_hexfile_from_blocks(blocks, &provider)?;
        let pipeline = self.build_pipeline_from_blocks(hexfile, &provider, blocks)?;
        let result = pipeline
            .execute(random_fill_bytes, |path| load_block(blocks, path))
            .map_err(|e| match e {
                PipelineError::Ops(err) => CliError::Other(err.to_string()),
                PipelineError::Log(err) => CliError::Other(format!("/L: {err}")),
        })?;
        let mut hexfile = result.hexfile;
        let checksum_bytes = self.apply_checksum(&mut hexfile)?;
        let _signature_bytes = self.apply_data_processing(&mut hexfile)?;
        self.apply_signature_verification(&hexfile)?;
        self.write_outputs(&hexfile, &provider)?;

        Ok(ExecuteOutput { checksum_bytes })
    }

    fn build_pipeline<P: ReadProvider>(
        &self,
        hexfile: crate::HexFile,
        provider: &P,
    ) -> Result<Pipeline, CliError> {
        let log_commands = if let Some(ref path) = self.log_file {
            let content = provider
                .read_string(path)
                .map_err(|e| CliError::Other(format!("/L: {e}")))?;
            Some(
                crate::parse_log_commands(&content)
                    .map_err(|e| CliError::Other(format!("/L: {e}")))?,
            )
        } else {
            None
        };

        let mut merge_transparent = Vec::with_capacity(self.merge_transparent.len());
        for merge in &self.merge_transparent {
            let other = load_input(provider, &merge.file)?;
            merge_transparent.push(PipelineMerge {
                other,
                offset: merge.offset.unwrap_or(0),
                range: merge.range,
            });
        }
        let mut merge_opaque = Vec::with_capacity(self.merge_opaque.len());
        for merge in &self.merge_opaque {
            let other = load_input(provider, &merge.file)?;
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

        Ok(Pipeline {
            hexfile,
            fill_ranges: self.fill_ranges.clone(),
            fill_pattern: if self.fill_pattern_set {
                Some(self.fill_pattern.clone())
            } else {
                None
            },
            cut_ranges: self.cut_ranges.clone(),
            merge_transparent,
            merge_opaque,
            address_ranges: self.address_range.clone(),
            log_commands,
            fill_all: if self.fill_all {
                Some(self.align_fill)
            } else {
                None
            },
            align,
            split: self.split_block_size,
            swap_word: self.swap_word,
            swap_long: self.swap_long,
            checksum: None,
            map_star12: self.s12_map,
            map_star12x: self.s12x_map,
            map_star08: self.s08_map,
            remap: self.remap.as_ref().map(|remap| RemapOptions {
                start: remap.start,
                end: remap.end,
                linear: remap.linear,
                size: remap.size,
                inc: remap.inc,
            }),
            dspic_expand: self
                .dspic_expand
                .iter()
                .map(|op| PipelineDspic {
                    range: op.range,
                    target: op.target,
                })
                .collect(),
            dspic_shrink: self
                .dspic_shrink
                .iter()
                .map(|op| PipelineDspic {
                    range: op.range,
                    target: op.target,
                })
                .collect(),
            dspic_clear_ghost: self.dspic_clear_ghost.clone(),
        })
    }

    fn build_pipeline_from_blocks<P: ReadProvider>(
        &self,
        hexfile: crate::HexFile,
        provider: &P,
        blocks: &HashMap<String, crate::HexFile>,
    ) -> Result<Pipeline, CliError> {
        let log_commands = if let Some(ref path) = self.log_file {
            let content = provider
                .read_string(path)
                .map_err(|e| CliError::Other(format!("/L: {e}")))?;
            Some(
                crate::parse_log_commands(&content)
                    .map_err(|e| CliError::Other(format!("/L: {e}")))?,
            )
        } else {
            None
        };

        let mut merge_transparent = Vec::with_capacity(self.merge_transparent.len());
        for merge in &self.merge_transparent {
            let other = load_block(blocks, &merge.file)?;
            merge_transparent.push(PipelineMerge {
                other,
                offset: merge.offset.unwrap_or(0),
                range: merge.range,
            });
        }
        let mut merge_opaque = Vec::with_capacity(self.merge_opaque.len());
        for merge in &self.merge_opaque {
            let other = load_block(blocks, &merge.file)?;
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

        Ok(Pipeline {
            hexfile,
            fill_ranges: self.fill_ranges.clone(),
            fill_pattern: if self.fill_pattern_set {
                Some(self.fill_pattern.clone())
            } else {
                None
            },
            cut_ranges: self.cut_ranges.clone(),
            merge_transparent,
            merge_opaque,
            address_ranges: self.address_range.clone(),
            log_commands,
            fill_all: if self.fill_all {
                Some(self.align_fill)
            } else {
                None
            },
            align,
            split: self.split_block_size,
            swap_word: self.swap_word,
            swap_long: self.swap_long,
            checksum: None,
            map_star12: self.s12_map,
            map_star12x: self.s12x_map,
            map_star08: self.s08_map,
            remap: self.remap.as_ref().map(|remap| RemapOptions {
                start: remap.start,
                end: remap.end,
                linear: remap.linear,
                size: remap.size,
                inc: remap.inc,
            }),
            dspic_expand: self
                .dspic_expand
                .iter()
                .map(|op| PipelineDspic {
                    range: op.range,
                    target: op.target,
                })
                .collect(),
            dspic_shrink: self
                .dspic_shrink
                .iter()
                .map(|op| PipelineDspic {
                    range: op.range,
                    target: op.target,
                })
                .collect(),
            dspic_clear_ghost: self.dspic_clear_ghost.clone(),
        })
    }

    fn load_hexfile<P: ReadProvider>(&self, provider: &P) -> Result<crate::HexFile, CliError> {
        if let Some(ref import) = self.import_binary {
            return load_binary_input(provider, &import.file, import.offset);
        }
        if let Some(ref import) = self.import_hex_ascii {
            let ascii = load_hex_ascii_input(provider, &import.file, import.offset)?;
            if let Some(ref path) = self.input_file {
                let mut base = load_input(provider, path)?;
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
            return load_intel_hex_16bit_input(provider, import);
        }
        if let Some(ref path) = self.input_file {
            return load_input(provider, path);
        }
        if self.log_file.is_some() {
            return Ok(crate::HexFile::new());
        }
        Err(ParseArgError::MissingInputFile.into())
    }

    fn load_hexfile_from_blocks(
        &self,
        blocks: &HashMap<String, crate::HexFile>,
        provider: &impl ReadProvider,
    ) -> Result<crate::HexFile, CliError> {
        if let Some(ref import) = self.import_binary {
            return load_binary_input(provider, &import.file, import.offset);
        }
        if let Some(ref import) = self.import_hex_ascii {
            let ascii = load_hex_ascii_input(provider, &import.file, import.offset)?;
            if let Some(ref path) = self.input_file {
                let mut base = load_block(blocks, path)?;
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
            return load_intel_hex_16bit_input(provider, import);
        }
        if let Some(ref path) = self.input_file {
            return load_block(blocks, path);
        }
        if self.log_file.is_some() {
            return Ok(crate::HexFile::new());
        }
        Err(ParseArgError::MissingInputFile.into())
    }

    fn apply_checksum(&self, hexfile: &mut crate::HexFile) -> Result<Option<Vec<u8>>, CliError> {
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
            .map(|forced| crate::ForcedRange {
                range: forced.range,
                pattern: forced.pattern.clone(),
            });
        let lib_target = match &cs_params.target {
            ChecksumTarget::Address(addr) => crate::ChecksumTarget::Address(*addr),
            ChecksumTarget::Append => crate::ChecksumTarget::Append,
            ChecksumTarget::Begin => {
                if let Some(start) = hexfile.min_address() {
                    crate::ChecksumTarget::Address(start)
                } else {
                    crate::ChecksumTarget::Append
                }
            }
            ChecksumTarget::Prepend => crate::ChecksumTarget::Prepend,
            ChecksumTarget::OverwriteEnd => crate::ChecksumTarget::OverwriteEnd,
            ChecksumTarget::File(path) => crate::ChecksumTarget::File(path.clone()),
        };
        let result = self.wrap_error(
            &opt,
            crate::flag_checksum(
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

    fn apply_data_processing(&self, hexfile: &mut crate::HexFile) -> Result<Option<Vec<u8>>, CliError> {
        let Some(ref params) = self.data_processing else {
            return Ok(None);
        };
        apply_data_processing(hexfile, params)
    }

    fn apply_signature_verification(&self, hexfile: &crate::HexFile) -> Result<(), CliError> {
        let Some(ref params) = self.signature_verify else {
            return Ok(());
        };
        apply_signature_verification(hexfile, params)
    }

    fn write_outputs<P: ReadProvider>(
        &self,
        hexfile: &crate::HexFile,
        provider: &P,
    ) -> Result<(), CliError> {
        write_output_for_args(self, hexfile, provider)
    }
}

fn random_fill_bytes(range: Range) -> Vec<u8> {
    let seed = crate::random_fill_seed_from_time(range);
    crate::random_fill_bytes(range, seed)
}

fn load_block(
    blocks: &HashMap<String, crate::HexFile>,
    path: &Path,
) -> Result<crate::HexFile, CliError> {
    let key = path.to_string_lossy().to_string();
    if let Some(block) = blocks.get(&key) {
        return Ok(block.clone());
    }

    let provider = FsProvider;
    load_input(&provider, path)
}

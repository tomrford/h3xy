use h3xy::{ChecksumAlgorithm, Range, RemapOptions};

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
        if self.s08_map {
            return Err(CliError::Unsupported(
                "S08 address mapping is not supported yet".into(),
            ));
        }
        if self.s12_map && self.s12x_map {
            return Err(CliError::Unsupported(
                "cannot combine /S12MAP and /S12XMAP".into(),
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
        if !self.dspic_expand.is_empty() {
            return Err(CliError::Unsupported(
                "dsPIC expand is not supported yet".into(),
            ));
        }
        if !self.dspic_shrink.is_empty() {
            return Err(CliError::Unsupported(
                "dsPIC shrink is not supported yet".into(),
            ));
        }
        if !self.dspic_clear_ghost.is_empty() {
            return Err(CliError::Unsupported(
                "dsPIC clear ghost is not supported yet".into(),
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
        if (self.import_binary.is_some()
            || self.import_hex_ascii.is_some()
            || self.import_i16.is_some())
            && self.input_file.is_some()
        {
            return Err(CliError::Unsupported(
                "explicit import (/IN, /IA, /II2) cannot be combined with input file".into(),
            ));
        }
        Ok(())
    }

    /// Execute the parsed arguments in HexView processing order.
    pub fn execute(&self) -> Result<ExecuteOutput, CliError> {
        self.validate_supported_features()?;

        let mut hexfile = self.load_hexfile()?;
        self.apply_mappings(&mut hexfile)?;
        self.apply_fill(&mut hexfile);
        self.apply_cut(&mut hexfile);
        self.apply_merge(&mut hexfile)?;
        self.apply_filter_ranges(&mut hexfile);
        self.apply_log(&mut hexfile)?;
        self.apply_fill_all(&mut hexfile);
        self.apply_align(&mut hexfile)?;
        self.apply_split(&mut hexfile);
        self.apply_swap(&mut hexfile)?;
        let checksum_bytes = self.apply_checksum(&mut hexfile)?;
        self.write_outputs(&hexfile)?;

        Ok(ExecuteOutput { checksum_bytes })
    }

    fn load_hexfile(&self) -> Result<h3xy::HexFile, CliError> {
        if let Some(ref import) = self.import_binary {
            return super::io::load_binary_input(&import.file, import.offset);
        }
        if let Some(ref import) = self.import_hex_ascii {
            return super::io::load_hex_ascii_input(&import.file, import.offset);
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

    fn apply_mappings(&self, hexfile: &mut h3xy::HexFile) -> Result<(), CliError> {
        if self.s12_map {
            self.wrap_error("/S12MAP", h3xy::flag_map_star12(hexfile))?;
        }
        if self.s12x_map {
            self.wrap_error("/S12XMAP", h3xy::flag_map_star12x(hexfile))?;
        }
        if let Some(ref remap) = self.remap {
            let options = RemapOptions {
                start: remap.start,
                end: remap.end,
                linear: remap.linear,
                size: remap.size,
                inc: remap.inc,
            };
            self.wrap_error("/REMAP", h3xy::flag_remap(hexfile, &options))?;
        }
        Ok(())
    }

    fn apply_fill(&self, hexfile: &mut h3xy::HexFile) {
        if self.fill_pattern_set {
            h3xy::flag_fill_ranges_pattern(hexfile, &self.fill_ranges, &self.fill_pattern);
            return;
        }
        h3xy::flag_fill_ranges_random(hexfile, &self.fill_ranges, random_fill_bytes);
    }

    fn apply_cut(&self, hexfile: &mut h3xy::HexFile) {
        h3xy::flag_cut_ranges(hexfile, &self.cut_ranges);
    }

    fn apply_merge(&self, hexfile: &mut h3xy::HexFile) -> Result<(), CliError> {
        for merge in &self.merge_transparent {
            let other = load_input(&merge.file)?;
            let opt = format!("/MT:{}", merge.file.display());
            self.wrap_error(
                &opt,
                h3xy::flag_merge_transparent(
                    hexfile,
                    &other,
                    merge.offset.unwrap_or(0),
                    merge.range,
                ),
            )?;
        }
        for merge in &self.merge_opaque {
            let other = load_input(&merge.file)?;
            let opt = format!("/MO:{}", merge.file.display());
            self.wrap_error(
                &opt,
                h3xy::flag_merge_opaque(hexfile, &other, merge.offset.unwrap_or(0), merge.range),
            )?;
        }
        Ok(())
    }

    fn apply_filter_ranges(&self, hexfile: &mut h3xy::HexFile) {
        h3xy::flag_filter_ranges(hexfile, &self.address_range);
    }

    fn apply_log(&self, hexfile: &mut h3xy::HexFile) -> Result<(), CliError> {
        if let Some(ref log_path) = self.log_file {
            self.wrap_error(
                "/L",
                h3xy::flag_execute_log_file(hexfile, log_path, load_input),
            )?;
        }
        Ok(())
    }

    fn apply_fill_all(&self, hexfile: &mut h3xy::HexFile) {
        if self.fill_all {
            h3xy::flag_fill_all(hexfile, self.align_fill);
        }
    }

    fn apply_align(&self, hexfile: &mut h3xy::HexFile) -> Result<(), CliError> {
        if let Some(alignment) = self.align_address {
            self.wrap_error(
                "/AD/AL",
                h3xy::flag_align(hexfile, alignment, self.align_fill, self.align_length),
            )?;
        }
        Ok(())
    }

    fn apply_split(&self, hexfile: &mut h3xy::HexFile) {
        if let Some(size) = self.split_block_size {
            h3xy::flag_split(hexfile, size);
        }
    }

    fn apply_swap(&self, hexfile: &mut h3xy::HexFile) -> Result<(), CliError> {
        if self.swap_word {
            self.wrap_error("/SWAPWORD", h3xy::flag_swap_word(hexfile))?;
        }
        if self.swap_long {
            self.wrap_error("/SWAPLONG", h3xy::flag_swap_long(hexfile))?;
        }
        Ok(())
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

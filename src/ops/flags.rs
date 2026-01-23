use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    AlignOptions, ChecksumAlgorithm, ChecksumOptions, ChecksumTarget, FillOptions, ForcedRange,
    HexFile, MergeMode, MergeOptions, Range, RemapOptions, Segment,
};

use super::{LogError, OpsError, execute_log_file};

/// CLI: /FR with /FP (fill ranges with explicit pattern).
pub fn flag_fill_ranges_pattern(hexfile: &mut HexFile, ranges: &[Range], pattern: &[u8]) {
    if ranges.is_empty() || pattern.is_empty() {
        return;
    }
    let options = FillOptions {
        pattern: pattern.to_vec(),
        overwrite: false,
    };
    for range in ranges {
        hexfile.fill(*range, &options);
    }
}

/// CLI: /FR with /FP (fill ranges with explicit pattern).
pub fn flag_fill_ranges_random<F>(hexfile: &mut HexFile, ranges: &[Range], mut random: F)
where
    F: FnMut(Range) -> Vec<u8>,
{
    for range in ranges {
        let data = random(*range);
        hexfile.prepend_segment(Segment::new(range.start(), data));
    }
}

/// CLI: /FR without /FP (random fill helper).
pub fn random_fill_bytes(range: Range, seed: u64) -> Vec<u8> {
    let len = range.length() as usize;
    if len == 0 {
        return Vec::new();
    }
    let mut state = seed;
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        out.push((state >> 32) as u8);
    }
    out
}

/// CLI: /FR without /FP (seed helper for random fill).
pub fn random_fill_seed_from_time(range: Range) -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let seed = now ^ ((range.start() as u64) << 32) ^ (range.length() as u64);
    if seed == 0 { 0x9E3779B97F4A7C15 } else { seed }
}

/// CLI: /CR (cut/remove ranges).
pub fn flag_cut_ranges(hexfile: &mut HexFile, ranges: &[Range]) {
    hexfile.cut_ranges(ranges);
}

/// CLI: /MT (transparent merge).
pub fn flag_merge_transparent(
    hexfile: &mut HexFile,
    other: &HexFile,
    offset: i64,
    range: Option<Range>,
) -> Result<(), OpsError> {
    let options = MergeOptions {
        mode: MergeMode::Preserve,
        offset,
        range,
    };
    hexfile
        .merge(other, &options)
        .map_err(|e| e.with_context("/MT"))
}

/// CLI: /MO (opaque merge).
pub fn flag_merge_opaque(
    hexfile: &mut HexFile,
    other: &HexFile,
    offset: i64,
    range: Option<Range>,
) -> Result<(), OpsError> {
    let options = MergeOptions {
        mode: MergeMode::Overwrite,
        offset,
        range,
    };
    hexfile
        .merge(other, &options)
        .map_err(|e| e.with_context("/MO"))
}

/// CLI: /AR (filter/keep ranges).
pub fn flag_filter_ranges(hexfile: &mut HexFile, ranges: &[Range]) {
    if !ranges.is_empty() {
        hexfile.filter_ranges(ranges);
    }
}

/// CLI: /FA (fill all gaps with /AF byte).
pub fn flag_fill_all(hexfile: &mut HexFile, fill_byte: u8) {
    hexfile.fill_gaps(fill_byte);
}

/// CLI: /AD, /AL (align), uses /AF as fill.
pub fn flag_align(
    hexfile: &mut HexFile,
    alignment: u32,
    fill_byte: u8,
    align_length: bool,
) -> Result<(), OpsError> {
    let options = AlignOptions {
        alignment,
        fill_byte,
        align_length,
    };
    hexfile
        .align(&options)
        .map_err(|e| e.with_context("/AD/AL"))
}

/// CLI: /SB (split block size).
pub fn flag_split(hexfile: &mut HexFile, size: u32) {
    hexfile.split(size);
}

/// CLI: /SWAPWORD.
pub fn flag_swap_word(hexfile: &mut HexFile) -> Result<(), OpsError> {
    hexfile
        .swap_bytes(crate::SwapMode::Word)
        .map_err(|e| e.with_context("/SWAPWORD"))
}

/// CLI: /SWAPLONG.
pub fn flag_swap_long(hexfile: &mut HexFile) -> Result<(), OpsError> {
    hexfile
        .swap_bytes(crate::SwapMode::DWord)
        .map_err(|e| e.with_context("/SWAPLONG"))
}

/// CLI: /REMAP.
pub fn flag_remap(hexfile: &mut HexFile, options: &RemapOptions) -> Result<(), OpsError> {
    hexfile.remap(options).map_err(|e| e.with_context("/REMAP"))
}

/// CLI: /S12MAP.
pub fn flag_map_star12(hexfile: &mut HexFile) -> Result<(), OpsError> {
    hexfile.map_star12().map_err(|e| e.with_context("/S12MAP"))
}

/// CLI: /S12XMAP.
pub fn flag_map_star12x(hexfile: &mut HexFile) -> Result<(), OpsError> {
    hexfile
        .map_star12x()
        .map_err(|e| e.with_context("/S12XMAP"))
}

/// CLI: /S08MAP.
pub fn flag_map_star08(hexfile: &mut HexFile) -> Result<(), OpsError> {
    hexfile.map_star08().map_err(|e| e.with_context("/S08MAP"))
}

/// CLI: /CDSPX.
pub fn flag_dspic_expand(
    hexfile: &mut HexFile,
    range: Range,
    target: Option<u32>,
) -> Result<(), OpsError> {
    hexfile
        .dspic_expand(range, target)
        .map_err(|e| e.with_context("/CDSPX"))
}

/// CLI: /CDSPS.
pub fn flag_dspic_shrink(
    hexfile: &mut HexFile,
    range: Range,
    target: Option<u32>,
) -> Result<(), OpsError> {
    hexfile
        .dspic_shrink(range, target)
        .map_err(|e| e.with_context("/CDSPS"))
}

/// CLI: /CDSPG.
pub fn flag_dspic_clear_ghost(hexfile: &mut HexFile, range: Range) -> Result<(), OpsError> {
    hexfile
        .dspic_clear_ghost(range)
        .map_err(|e| e.with_context("/CDSPG"))
}

/// CLI: /CS or /CSR (little-endian output).
pub fn flag_checksum(
    hexfile: &mut HexFile,
    algorithm: ChecksumAlgorithm,
    range: Option<Range>,
    little_endian_output: bool,
    forced_range: Option<ForcedRange>,
    exclude_ranges: &[Range],
    target: &ChecksumTarget,
) -> Result<Vec<u8>, OpsError> {
    let context = if little_endian_output { "/CSR" } else { "/CS" };
    let options = ChecksumOptions {
        algorithm,
        range,
        little_endian_output,
        forced_range,
        exclude_ranges: exclude_ranges.to_vec(),
    };
    hexfile
        .checksum(&options, target)
        .map_err(|e| e.with_context(context))
}

/// CLI: /L (execute log file commands).
pub fn flag_execute_log_file<F, E>(
    hexfile: &mut HexFile,
    path: &std::path::Path,
    load: F,
) -> Result<(), LogError>
where
    F: FnMut(&std::path::Path) -> Result<HexFile, E>,
    E: Into<Box<dyn std::error::Error>>,
{
    execute_log_file(hexfile, path, load)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_fill_bytes_deterministic() {
        let range = Range::from_start_length(0x1000, 4).unwrap();
        let data = random_fill_bytes(range, 1);
        assert_eq!(data, vec![0x2D, 0xCF, 0x46, 0x29]);
    }
}

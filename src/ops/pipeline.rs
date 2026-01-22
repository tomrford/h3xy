use std::path::Path;

use thiserror::Error;

use crate::{
    AlignOptions, ChecksumAlgorithm, ChecksumTarget, ForcedRange, HexFile, Range, RemapOptions,
};

use super::{
    LogCommand, LogError, OpsError, execute_log_commands, flag_align, flag_checksum,
    flag_cut_ranges, flag_fill_all, flag_fill_ranges_pattern, flag_fill_ranges_random,
    flag_filter_ranges, flag_map_star08, flag_map_star12, flag_map_star12x, flag_merge_opaque,
    flag_merge_transparent, flag_remap, flag_split, flag_swap_long, flag_swap_word,
};

#[derive(Debug, Clone)]
pub struct PipelineMerge {
    pub other: HexFile,
    pub offset: i64,
    pub range: Option<Range>,
}

#[derive(Debug, Clone)]
pub struct PipelineChecksum {
    pub algorithm: ChecksumAlgorithm,
    pub range: Option<Range>,
    pub little_endian_output: bool,
    pub forced_range: Option<ForcedRange>,
    pub exclude_ranges: Vec<Range>,
    pub target: ChecksumTarget,
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub hexfile: HexFile,
    pub fill_ranges: Vec<Range>,
    pub fill_pattern: Option<Vec<u8>>,
    pub cut_ranges: Vec<Range>,
    pub merge_transparent: Vec<PipelineMerge>,
    pub merge_opaque: Vec<PipelineMerge>,
    pub address_ranges: Vec<Range>,
    pub log_commands: Option<Vec<LogCommand>>,
    pub fill_all: Option<u8>,
    pub align: Option<AlignOptions>,
    pub split: Option<u32>,
    pub swap_word: bool,
    pub swap_long: bool,
    pub checksum: Option<PipelineChecksum>,
    pub map_star12: bool,
    pub map_star12x: bool,
    pub map_star08: bool,
    pub remap: Option<RemapOptions>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self {
            hexfile: HexFile::new(),
            fill_ranges: Vec::new(),
            fill_pattern: None,
            cut_ranges: Vec::new(),
            merge_transparent: Vec::new(),
            merge_opaque: Vec::new(),
            address_ranges: Vec::new(),
            log_commands: None,
            fill_all: None,
            align: None,
            split: None,
            swap_word: false,
            swap_long: false,
            checksum: None,
            map_star12: false,
            map_star12x: false,
            map_star08: false,
            remap: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error(transparent)]
    Ops(#[from] OpsError),
    #[error(transparent)]
    Log(#[from] LogError),
}

#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub hexfile: HexFile,
    pub checksum_bytes: Option<Vec<u8>>,
}

impl Pipeline {
    pub fn execute<F, L, E>(
        self,
        mut random_fill: F,
        mut log_loader: L,
    ) -> Result<PipelineResult, PipelineError>
    where
        F: FnMut(Range) -> Vec<u8>,
        L: FnMut(&Path) -> Result<HexFile, E>,
        E: Into<Box<dyn std::error::Error>>,
    {
        let mut hexfile = self.hexfile;

        if self.map_star12 {
            flag_map_star12(&mut hexfile)?;
        }
        if self.map_star12x {
            flag_map_star12x(&mut hexfile)?;
        }
        if self.map_star08 {
            flag_map_star08(&mut hexfile)?;
        }
        if let Some(ref remap) = self.remap {
            flag_remap(&mut hexfile, remap)?;
        }

        if let Some(ref pattern) = self.fill_pattern {
            flag_fill_ranges_pattern(&mut hexfile, &self.fill_ranges, pattern);
        } else {
            flag_fill_ranges_random(&mut hexfile, &self.fill_ranges, &mut random_fill);
        }

        flag_cut_ranges(&mut hexfile, &self.cut_ranges);

        for merge in &self.merge_transparent {
            flag_merge_transparent(&mut hexfile, &merge.other, merge.offset, merge.range)?;
        }
        for merge in &self.merge_opaque {
            flag_merge_opaque(&mut hexfile, &merge.other, merge.offset, merge.range)?;
        }

        flag_filter_ranges(&mut hexfile, &self.address_ranges);

        if let Some(ref commands) = self.log_commands {
            execute_log_commands(&mut hexfile, commands, &mut log_loader)?;
        }

        if let Some(fill_byte) = self.fill_all {
            flag_fill_all(&mut hexfile, fill_byte);
        }

        if let Some(ref align) = self.align {
            flag_align(
                &mut hexfile,
                align.alignment,
                align.fill_byte,
                align.align_length,
            )?;
        }

        if let Some(size) = self.split {
            flag_split(&mut hexfile, size);
        }

        if self.swap_word {
            flag_swap_word(&mut hexfile)?;
        }
        if self.swap_long {
            flag_swap_long(&mut hexfile)?;
        }

        let checksum_bytes = if let Some(ref checksum) = self.checksum {
            Some(flag_checksum(
                &mut hexfile,
                checksum.algorithm,
                checksum.range,
                checksum.little_endian_output,
                checksum.forced_range.clone(),
                &checksum.exclude_ranges,
                &checksum.target,
            )?)
        } else {
            None
        };

        Ok(PipelineResult {
            hexfile,
            checksum_bytes,
        })
    }

    pub fn execute_without_log<F>(self, random_fill: F) -> Result<PipelineResult, PipelineError>
    where
        F: FnMut(Range) -> Vec<u8>,
    {
        self.execute(random_fill, |_| {
            Err(std::io::Error::other("log loader not provided"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Segment;

    #[test]
    fn test_pipeline_fill_cut_align() {
        let hexfile = HexFile::with_segments(vec![Segment::new(0x1001, vec![0xAA, 0xBB])]);
        let pipeline = Pipeline {
            hexfile,
            fill_ranges: vec![Range::from_start_length(0x1000, 4).unwrap()],
            fill_pattern: Some(vec![0xFF]),
            cut_ranges: vec![Range::from_start_end(0x1002, 0x1002).unwrap()],
            align: Some(AlignOptions {
                alignment: 4,
                fill_byte: 0x00,
                align_length: true,
            }),
            ..Default::default()
        };

        let result = pipeline
            .execute_without_log(|range| vec![0x00; range.length() as usize])
            .unwrap();
        let norm = result.hexfile.normalized_lossy();
        assert_eq!(norm.segments().len(), 1);
        assert_eq!(norm.segments()[0].start_address, 0x1000);
        assert_eq!(norm.segments()[0].data.len(), 4);
    }
}

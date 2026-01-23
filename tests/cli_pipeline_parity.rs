mod common;

use common::{assert_success, run_h3xy, temp_dir, write_file};
use h3xy::{
    AlignOptions, BinaryWriteOptions, ChecksumAlgorithm, ChecksumTarget, IntelHexWriteOptions,
    Pipeline, PipelineMerge, Range, parse_binary, write_binary, write_intel_hex,
};

#[test]
fn test_cli_pipeline_parity_basic_chain() {
    let dir = temp_dir("cli_pipeline_parity");
    let base = dir.join("base.bin");
    let merge = dir.join("merge.bin");
    let out_cli = dir.join("out.hex");

    write_file(&base, &[0x10, 0x11, 0x12, 0x13]);
    write_file(&merge, &[0xAA, 0xBB]);

    let args = vec![
        format!("/IN:{};0x1000", base.display()),
        "/FR:0x1000-0x100F".to_string(),
        "/FP:F0".to_string(),
        "/CR:0x1004-0x1005".to_string(),
        format!("/MT:{};0x1008", merge.display()),
        "/AR:0x1000-0x1010".to_string(),
        "/AF:00".to_string(),
        "/AD4".to_string(),
        "/AL".to_string(),
        "/SWAPWORD".to_string(),
        "/SB:4".to_string(),
        "/XI".to_string(),
        "-o".to_string(),
        out_cli.display().to_string(),
    ];

    let output = run_h3xy(&args);
    assert_success(&output);
    let cli_bytes = std::fs::read(&out_cli).unwrap();

    let base_hex = parse_binary(&[0x10, 0x11, 0x12, 0x13], 0x1000).unwrap();
    let merge_hex = parse_binary(&[0xAA, 0xBB], 0).unwrap();

    let mut pipeline = Pipeline::default();
    pipeline.hexfile = base_hex;
    pipeline.fill_ranges = vec![Range::from_start_end(0x1000, 0x100F).unwrap()];
    pipeline.fill_pattern = Some(vec![0xF0]);
    pipeline.cut_ranges = vec![Range::from_start_end(0x1004, 0x1005).unwrap()];
    pipeline.merge_transparent = vec![PipelineMerge {
        other: merge_hex,
        offset: 0x1008,
        range: None,
    }];
    pipeline.address_ranges = vec![Range::from_start_end(0x1000, 0x1010).unwrap()];
    pipeline.align = Some(AlignOptions {
        alignment: 4,
        fill_byte: 0x00,
        align_length: true,
    });
    pipeline.split = Some(4);
    pipeline.swap_word = true;

    let result = pipeline.execute_without_log(|range| vec![0; range.length() as usize]).unwrap();
    let lib_bytes = write_intel_hex(&result.hexfile, &IntelHexWriteOptions::default());

    assert_eq!(cli_bytes, lib_bytes);
}

#[test]
fn test_cli_pipeline_parity_binary_order() {
    let dir = temp_dir("cli_pipeline_parity_xn");
    let base = dir.join("base.bin");
    let merge = dir.join("merge.bin");
    let out_cli = dir.join("out.bin");

    write_file(&base, &[0x01, 0x02]);
    write_file(&merge, &[0xAA, 0xBB]);

    let args = vec![
        format!("/IN:{};0x2000", base.display()),
        format!("/MT:{};0x1000", merge.display()),
        "/XN".to_string(),
        "-o".to_string(),
        out_cli.display().to_string(),
    ];

    let output = run_h3xy(&args);
    assert_success(&output);
    let cli_bytes = std::fs::read(&out_cli).unwrap();

    let base_hex = parse_binary(&[0x01, 0x02], 0x2000).unwrap();
    let merge_hex = parse_binary(&[0xAA, 0xBB], 0).unwrap();

    let mut pipeline = Pipeline::default();
    pipeline.hexfile = base_hex;
    pipeline.merge_transparent = vec![PipelineMerge {
        other: merge_hex,
        offset: 0x1000,
        range: None,
    }];

    let result = pipeline.execute_without_log(|range| vec![0; range.length() as usize]).unwrap();
    let lib_bytes = write_binary(&result.hexfile, &BinaryWriteOptions::default());

    assert_eq!(cli_bytes, lib_bytes);
}

#[test]
fn test_cli_checksum_parity_begin() {
    let dir = temp_dir("cli_pipeline_parity_cs");
    let base = dir.join("base.bin");
    let out_cli = dir.join("out.hex");

    write_file(&base, &[0x01, 0x02, 0x03, 0x04]);

    let args = vec![
        format!("/IN:{};0x1000", base.display()),
        "/CS0:@BEGIN".to_string(),
        "/XI".to_string(),
        "-o".to_string(),
        out_cli.display().to_string(),
    ];

    let output = run_h3xy(&args);
    assert_success(&output);
    let cli_bytes = std::fs::read(&out_cli).unwrap();

    let mut hexfile = parse_binary(&[0x01, 0x02, 0x03, 0x04], 0x1000).unwrap();
    let start = hexfile.min_address().unwrap();
    let algorithm = ChecksumAlgorithm::from_index(0).unwrap();
    let _ = h3xy::flag_checksum(
        &mut hexfile,
        algorithm,
        None,
        false,
        None,
        &[],
        &ChecksumTarget::Address(start),
    )
    .unwrap();
    let lib_bytes = write_intel_hex(&hexfile, &IntelHexWriteOptions::default());

    assert_eq!(cli_bytes, lib_bytes);
}

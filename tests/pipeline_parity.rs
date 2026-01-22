mod common;

use h3xy::{AlignOptions, Pipeline, PipelineMerge, Range, parse_binary, parse_intel_hex};

use common::{assert_success, run_h3xy, temp_dir, write_file};

#[test]
fn test_pipeline_matches_cli_basic() {
    let dir = temp_dir("pipeline_parity");
    let input_path = dir.join("input.bin");
    let merge_path = dir.join("merge.bin");
    let output_path = dir.join("out.hex");

    write_file(&input_path, &[0x01, 0x02, 0x03, 0x04]);
    write_file(&merge_path, &[0x99, 0x88]);

    let args = vec![
        input_path.to_string_lossy().to_string(),
        "/FR:0x0-0x7".to_string(),
        "/FP:AA".to_string(),
        "/CR:0x2-0x3".to_string(),
        format!("/MO:{};0x6", merge_path.to_string_lossy()),
        "/AR:0x0-0x7".to_string(),
        "/AD:4".to_string(),
        "/AL".to_string(),
        "/AF:0x00".to_string(),
        "-o".to_string(),
        output_path.to_string_lossy().to_string(),
    ];

    let output = run_h3xy(&args);
    assert_success(&output);

    let cli_bytes = std::fs::read(&output_path).unwrap();
    let cli_hexfile = parse_intel_hex(&cli_bytes).unwrap();

    let input_hex = parse_binary(&[0x01, 0x02, 0x03, 0x04], 0).unwrap();
    let merge_hex = parse_binary(&[0x99, 0x88], 0).unwrap();

    let pipeline = Pipeline {
        hexfile: input_hex,
        fill_ranges: vec![Range::from_start_end(0x0, 0x7).unwrap()],
        fill_pattern: Some(vec![0xAA]),
        cut_ranges: vec![Range::from_start_end(0x2, 0x3).unwrap()],
        merge_opaque: vec![PipelineMerge {
            other: merge_hex,
            offset: 0x6,
            range: None,
        }],
        address_ranges: vec![Range::from_start_end(0x0, 0x7).unwrap()],
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

    assert_eq!(
        result.hexfile.normalized_lossy(),
        cli_hexfile.normalized_lossy()
    );
}

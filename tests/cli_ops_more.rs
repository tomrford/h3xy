mod common;

use common::{assert_success, run_h3xy, temp_dir, write_file};
use h3xy::{
    HexFile, IntelHexMode, IntelHexWriteOptions, Segment, parse_intel_hex, write_intel_hex,
};

fn run_hex_output(args: Vec<String>, out_path: &std::path::Path) -> HexFile {
    let output = run_h3xy(&args);
    assert_success(&output);
    let data = std::fs::read(out_path).unwrap();
    parse_intel_hex(&data).unwrap()
}

#[test]
fn test_cli_cut_multiple_ranges_one_arg() {
    let dir = temp_dir("cli_cut_multi");
    let input = dir.join("input.bin");
    let out = dir.join("out.hex");
    write_file(&input, &[0, 1, 2, 3, 4, 5, 6, 7]);

    let args = vec![
        format!("/IN:{};0x5000", input.display()),
        "/CR:0x5001,0x1:0x5005-0x5006".to_string(),
        "/XI".to_string(),
        "-o".to_string(),
        out.display().to_string(),
    ];

    let hexfile = run_hex_output(args, &out);
    let mut segments = hexfile.segments().to_vec();
    segments.sort_by_key(|s| s.start_address);
    assert_eq!(segments.len(), 3);
    assert_eq!(segments[0].start_address, 0x5000);
    assert_eq!(segments[0].data, vec![0]);
    assert_eq!(segments[1].start_address, 0x5002);
    assert_eq!(segments[1].data, vec![2, 3, 4]);
    assert_eq!(segments[2].start_address, 0x5007);
    assert_eq!(segments[2].data, vec![7]);
}

#[test]
fn test_cli_merge_transparent_range_offset() {
    let dir = temp_dir("cli_mt");
    let base = dir.join("base.bin");
    let merge = dir.join("merge.bin");
    let out = dir.join("out.hex");
    write_file(&base, &[0x40, 0x41, 0x42, 0x43]);
    write_file(&merge, &[0xA0, 0xA1, 0xA2, 0xA3]);

    let args = vec![
        format!("/IN:{};0x4000", base.display()),
        format!("/MT:{};0x4000:0x0,0x4", merge.display()),
        "/XI".to_string(),
        "-o".to_string(),
        out.display().to_string(),
    ];

    let hexfile = run_hex_output(args, &out);
    let norm = hexfile.normalized_lossy();
    assert_eq!(
        norm.read_bytes_contiguous(0x4000, 4).unwrap(),
        vec![0x40, 0x41, 0x42, 0x43]
    );
}

#[test]
fn test_cli_merge_opaque_range_offset() {
    let dir = temp_dir("cli_mo");
    let base = dir.join("base.bin");
    let merge = dir.join("merge.bin");
    let out = dir.join("out.hex");
    write_file(&base, &[0x40, 0x41, 0x42, 0x43]);
    write_file(&merge, &[0xA0, 0xA1, 0xA2, 0xA3]);

    let args = vec![
        format!("/IN:{};0x4000", base.display()),
        format!("/MO:{};0x4000:0x0,0x4", merge.display()),
        "/XI".to_string(),
        "-o".to_string(),
        out.display().to_string(),
    ];

    let hexfile = run_hex_output(args, &out);
    let norm = hexfile.normalized_lossy();
    assert_eq!(
        norm.read_bytes_contiguous(0x4000, 4).unwrap(),
        vec![0xA0, 0xA1, 0xA2, 0xA3]
    );
}

#[test]
fn test_cli_mt_mo_conflict() {
    let dir = temp_dir("cli_mt_mo_conflict");
    let base = dir.join("base.bin");
    let merge = dir.join("merge.bin");
    let out = dir.join("out.hex");
    write_file(&base, &[0x00, 0x01]);
    write_file(&merge, &[0x10, 0x11]);

    let args = vec![
        format!("/IN:{};0x0", base.display()),
        format!("/MT:{};0x0", merge.display()),
        format!("/MO:{};0x0", merge.display()),
        "/XI".to_string(),
        "-o".to_string(),
        out.display().to_string(),
    ];

    let output = run_h3xy(&args);
    assert!(!output.status.success());
}

#[test]
fn test_cli_log_file_open_without_input() {
    let dir = temp_dir("cli_log_open");
    let input = dir.join("input.bin");
    let log = dir.join("commands.log");
    let out = dir.join("out.hex");
    write_file(&input, &[0xDE, 0xAD, 0xBE, 0xEF]);
    write_file(&log, format!("FileOpen {}", input.display()).as_bytes());

    let args = vec![
        format!("/L:{}", log.display()),
        "/XI".to_string(),
        "-o".to_string(),
        out.display().to_string(),
    ];

    let hexfile = run_hex_output(args, &out);
    let norm = hexfile.normalized_lossy();
    assert_eq!(norm.segments().len(), 1);
    assert_eq!(norm.segments()[0].start_address, 0x0);
    assert_eq!(norm.segments()[0].data, vec![0xDE, 0xAD, 0xBE, 0xEF]);
}

#[test]
fn test_cli_log_file_new_clears_data() {
    let dir = temp_dir("cli_log_new");
    let input = dir.join("input.bin");
    let log = dir.join("commands.log");
    let out = dir.join("out.hex");
    write_file(&input, &[0x01, 0x02, 0x03]);
    write_file(&log, b"FileNew");

    let args = vec![
        format!("/IN:{};0x1000", input.display()),
        format!("/L:{}", log.display()),
        "/XI".to_string(),
        "-o".to_string(),
        out.display().to_string(),
    ];

    let hexfile = run_hex_output(args, &out);
    assert!(hexfile.segments().is_empty());
}

#[test]
fn test_cli_log_file_invalid_command() {
    let dir = temp_dir("cli_log_invalid");
    let input = dir.join("input.bin");
    let log = dir.join("commands.log");
    let out = dir.join("out.hex");
    write_file(&input, &[0xAA]);
    write_file(&log, b"BogusCommand");

    let args = vec![
        format!("/IN:{};0x0", input.display()),
        format!("/L:{}", log.display()),
        "/XI".to_string(),
        "-o".to_string(),
        out.display().to_string(),
    ];

    let output = run_h3xy(&args);
    assert!(!output.status.success());
}

#[test]
fn test_cli_version_string_written_to_error_log() {
    let dir = temp_dir("cli_version_log");
    let input = dir.join("input.bin");
    let err = dir.join("error.log");
    let out = dir.join("out.hex");
    write_file(&input, &[0xAA, 0xBB]);

    let args = vec![
        format!("/IN:{};0x0", input.display()),
        format!("/E:{}", err.display()),
        "/V".to_string(),
        "/XN".to_string(),
        "-o".to_string(),
        out.display().to_string(),
    ];

    let output = run_h3xy(&args);
    assert_success(&output);
    let log = std::fs::read_to_string(&err).unwrap();
    assert_eq!(log, format!("Hexview V{}", env!("CARGO_PKG_VERSION")));
}

#[test]
fn test_cli_import_i16_scales_addresses() {
    let dir = temp_dir("cli_i16_import");
    let input = dir.join("input.hex");
    let out = dir.join("out.hex");
    let hex = b":02000100AABB98\n:00000001FF\n";
    write_file(&input, hex);

    let args = vec![
        format!("/II2={}", input.display()),
        "/XI".to_string(),
        "-o".to_string(),
        out.display().to_string(),
    ];

    let hexfile = run_hex_output(args, &out);
    let norm = hexfile.normalized_lossy();
    assert_eq!(norm.segments().len(), 1);
    assert_eq!(norm.segments()[0].start_address, 0x0002);
    assert_eq!(norm.segments()[0].data, vec![0xAA, 0xBB]);
}

#[test]
fn test_cli_s08map_examples() {
    let dir = temp_dir("cli_s08map");
    let input = dir.join("input.hex");
    let out = dir.join("out.hex");
    let hexfile = HexFile::with_segments(vec![
        Segment::new(0x4000, vec![0xAA]),
        Segment::new(0x028000, vec![0xBB]),
    ]);
    let data = write_intel_hex(&hexfile, &IntelHexWriteOptions::default());
    write_file(&input, &data);

    let args = vec![
        input.display().to_string(),
        "/S08MAP".to_string(),
        "/XI".to_string(),
        "-o".to_string(),
        out.display().to_string(),
    ];

    let hexfile = run_hex_output(args, &out);
    let mut segments = hexfile.segments().to_vec();
    segments.sort_by_key(|s| s.start_address);
    assert_eq!(segments[0].start_address, 0x104000);
    assert_eq!(segments[1].start_address, 0x108000);
}

#[test]
fn test_cli_dspic_expand_default_target() {
    let dir = temp_dir("cli_cdspx");
    let input = dir.join("input.bin");
    let out = dir.join("out.hex");
    write_file(&input, &[0xAA, 0xBB, 0xCC, 0xDD]);

    let args = vec![
        format!("/IN:{};0x1000", input.display()),
        "/CDSPX:0x1000,0x4".to_string(),
        "/XI".to_string(),
        "-o".to_string(),
        out.display().to_string(),
    ];

    let hexfile = run_hex_output(args, &out);
    let out_bytes = hexfile.read_bytes_contiguous(0x2000, 8).unwrap();
    assert_eq!(
        out_bytes,
        vec![0xAA, 0xBB, 0x00, 0x00, 0xCC, 0xDD, 0x00, 0x00]
    );
}

#[test]
fn test_cli_dspic_shrink_default_target() {
    let dir = temp_dir("cli_cdsps");
    let input = dir.join("input.bin");
    let out = dir.join("out.hex");
    write_file(&input, &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]);

    let args = vec![
        format!("/IN:{};0x2000", input.display()),
        "/CDSPS:0x2000,0x8".to_string(),
        "/XI".to_string(),
        "-o".to_string(),
        out.display().to_string(),
    ];

    let hexfile = run_hex_output(args, &out);
    let out_bytes = hexfile.read_bytes_contiguous(0x1000, 4).unwrap();
    assert_eq!(out_bytes, vec![0x11, 0x22, 0x55, 0x66]);
}

#[test]
fn test_cli_dspic_clear_ghost() {
    let dir = temp_dir("cli_cdspg");
    let input = dir.join("input.bin");
    let out = dir.join("out.hex");
    write_file(&input, &[0x01, 0x02, 0x03, 0xFF, 0x10, 0x11, 0x12, 0xEE]);

    let args = vec![
        format!("/IN:{};0x3000", input.display()),
        "/CDSPG:0x3000,0x8".to_string(),
        "/XI".to_string(),
        "-o".to_string(),
        out.display().to_string(),
    ];

    let hexfile = run_hex_output(args, &out);
    let out_bytes = hexfile.read_bytes_contiguous(0x3000, 8).unwrap();
    assert_eq!(
        out_bytes,
        vec![0x01, 0x02, 0x03, 0x00, 0x10, 0x11, 0x12, 0x00]
    );
}

#[test]
fn test_cli_remap_basic() {
    let dir = temp_dir("cli_remap");
    let input = dir.join("input.hex");
    let out = dir.join("out.hex");
    let hexfile = HexFile::with_segments(vec![
        Segment::new(0x1000, vec![0xAA]),
        Segment::new(0x018000, vec![0x01, 0x02]),
        Segment::new(0x028000, vec![0x03]),
    ]);
    let data = write_intel_hex(
        &hexfile,
        &IntelHexWriteOptions {
            bytes_per_line: 16,
            mode: IntelHexMode::ExtendedLinear,
        },
    );
    write_file(&input, &data);

    let args = vec![
        input.display().to_string(),
        "/remap:0x018000-0x02BFFF,0x008000,0x4000,0x010000".to_string(),
        "/XI".to_string(),
        "-o".to_string(),
        out.display().to_string(),
    ];

    let hexfile = run_hex_output(args, &out);
    let mut segments = hexfile.segments().to_vec();
    segments.sort_by_key(|s| s.start_address);
    assert_eq!(segments.len(), 3);
    assert_eq!(segments[0].start_address, 0x1000);
    assert_eq!(segments[1].start_address, 0x008000);
    assert_eq!(segments[2].start_address, 0x00C000);
}

#[test]
fn test_cli_remap_invalid_size() {
    let dir = temp_dir("cli_remap_invalid");
    let input = dir.join("input.bin");
    let out = dir.join("out.hex");
    write_file(&input, &[0xAA]);

    let args = vec![
        format!("/IN:{};0x018000", input.display()),
        "/remap:0x018000-0x01BFFF,0x0,0x0,0x010000".to_string(),
        "/XI".to_string(),
        "-o".to_string(),
        out.display().to_string(),
    ];

    let output = run_h3xy(&args);
    assert!(!output.status.success());
}

#[test]
fn test_cli_s12map_basic() {
    let dir = temp_dir("cli_s12map");
    let input = dir.join("input.hex");
    let out = dir.join("out.hex");
    let hexfile = HexFile::with_segments(vec![
        Segment::new(0x4000, vec![0xAA]),
        Segment::new(0xC000, vec![0xBB]),
        Segment::new(0x308000, vec![0x01]),
        Segment::new(0x318000, vec![0x02]),
    ]);
    let data = write_intel_hex(
        &hexfile,
        &IntelHexWriteOptions {
            bytes_per_line: 16,
            mode: IntelHexMode::ExtendedLinear,
        },
    );
    write_file(&input, &data);

    let args = vec![
        input.display().to_string(),
        "/s12map".to_string(),
        "/XI".to_string(),
        "-o".to_string(),
        out.display().to_string(),
    ];

    let hexfile = run_hex_output(args, &out);
    let mut segments = hexfile.segments().to_vec();
    segments.sort_by_key(|s| s.start_address);
    assert_eq!(segments.len(), 4);
    assert_eq!(segments[0].start_address, 0x0C0000);
    assert_eq!(segments[1].start_address, 0x0C4000);
    assert_eq!(segments[2].start_address, 0x0F8000);
    assert_eq!(segments[3].start_address, 0x0FC000);
}

#[test]
fn test_cli_s12xmap_basic() {
    let dir = temp_dir("cli_s12xmap");
    let input = dir.join("input.hex");
    let out = dir.join("out.hex");
    let hexfile = HexFile::with_segments(vec![
        Segment::new(0x4000, vec![0xAA]),
        Segment::new(0xC000, vec![0xBB]),
        Segment::new(0xE08000, vec![0x01]),
        Segment::new(0xE18000, vec![0x02]),
    ]);
    let data = write_intel_hex(
        &hexfile,
        &IntelHexWriteOptions {
            bytes_per_line: 16,
            mode: IntelHexMode::ExtendedLinear,
        },
    );
    write_file(&input, &data);

    let args = vec![
        input.display().to_string(),
        "/s12xmap".to_string(),
        "/XI".to_string(),
        "-o".to_string(),
        out.display().to_string(),
    ];

    let hexfile = run_hex_output(args, &out);
    let mut segments = hexfile.segments().to_vec();
    segments.sort_by_key(|s| s.start_address);
    assert_eq!(segments.len(), 4);
    assert_eq!(segments[0].start_address, 0x780000);
    assert_eq!(segments[1].start_address, 0x784000);
    assert_eq!(segments[2].start_address, 0x7F4000);
    assert_eq!(segments[3].start_address, 0x7FC000);
}

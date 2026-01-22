mod common;

use common::{assert_success, run_h3xy, temp_dir, write_file};
use h3xy::parse_intel_hex;

fn run_checksum_hex(input: &[u8], cs_arg: &str) -> h3xy::HexFile {
    let dir = temp_dir("cli_checksum");
    let input_path = dir.join("input.bin");
    let out_path = dir.join("out.hex");
    write_file(&input_path, input);

    let args = vec![
        format!("/IN:{};0x1000", input_path.display()),
        cs_arg.to_string(),
        "/XI".to_string(),
        "-o".to_string(),
        out_path.display().to_string(),
    ];

    let output = run_h3xy(&args);
    assert_success(&output);

    let data = std::fs::read(&out_path).unwrap();
    parse_intel_hex(&data).unwrap()
}

#[test]
fn test_cli_checksum_append() {
    let hexfile = run_checksum_hex(&[0x01, 0x02, 0x03, 0x04], "/CS0:@append");
    let norm = hexfile.normalized_lossy();
    assert_eq!(
        norm.read_bytes_contiguous(0x1000, 6).unwrap(),
        vec![0x01, 0x02, 0x03, 0x04, 0x00, 0x0A]
    );
}

#[test]
fn test_cli_checksum_upfront() {
    let hexfile = run_checksum_hex(&[0x01, 0x02, 0x03, 0x04], "/CS0:@upfront");
    let norm = hexfile.normalized_lossy();
    assert_eq!(
        norm.read_bytes_contiguous(0x0FFE, 6).unwrap(),
        vec![0x00, 0x0A, 0x01, 0x02, 0x03, 0x04]
    );
}

#[test]
fn test_cli_checksum_begin() {
    let hexfile = run_checksum_hex(&[0x01, 0x02, 0x03, 0x04], "/CS0:@begin");
    let norm = hexfile.normalized_lossy();
    assert_eq!(
        norm.read_bytes_contiguous(0x1000, 4).unwrap(),
        vec![0x00, 0x0A, 0x03, 0x04]
    );
}

#[test]
fn test_cli_checksum_overwrite_end() {
    let hexfile = run_checksum_hex(&[0x01, 0x02, 0x03, 0x04], "/CS0:@end");
    let norm = hexfile.normalized_lossy();
    assert_eq!(
        norm.read_bytes_contiguous(0x1000, 4).unwrap(),
        vec![0x01, 0x02, 0x00, 0x0A]
    );
}

#[test]
fn test_cli_checksum_address() {
    let hexfile = run_checksum_hex(&[0x01, 0x02, 0x03, 0x04], "/CS0:@0x1001");
    let norm = hexfile.normalized_lossy();
    assert_eq!(
        norm.read_bytes_contiguous(0x1000, 4).unwrap(),
        vec![0x01, 0x00, 0x0A, 0x04]
    );
}

#[test]
fn test_cli_checksum_limited_range() {
    let hexfile = run_checksum_hex(&[0x01, 0x02, 0x03, 0x04], "/CS0:@append;0x1000-0x1001");
    let norm = hexfile.normalized_lossy();
    assert_eq!(
        norm.read_bytes_contiguous(0x1000, 6).unwrap(),
        vec![0x01, 0x02, 0x03, 0x04, 0x00, 0x03]
    );
}

#[test]
fn test_cli_checksum_exclude_range() {
    let hexfile = run_checksum_hex(
        &[0x01, 0x02, 0x03, 0x04],
        "/CS0:@append;0x1000-0x1003/0x1001-0x1002",
    );
    let norm = hexfile.normalized_lossy();
    assert_eq!(
        norm.read_bytes_contiguous(0x1000, 6).unwrap(),
        vec![0x01, 0x02, 0x03, 0x04, 0x00, 0x05]
    );
}

#[test]
fn test_cli_checksum_forced_range_fill() {
    let hexfile = run_checksum_hex(&[0x01, 0x02], "/CS0:@append;!0x1000-0x1003#FF");
    let norm = hexfile.normalized_lossy();
    assert_eq!(
        norm.read_bytes_contiguous(0x1000, 2).unwrap(),
        vec![0x01, 0x02]
    );
    assert_eq!(
        norm.read_bytes_contiguous(0x1002, 2).unwrap(),
        vec![0x02, 0x01]
    );
}

#[test]
fn test_cli_checksum_little_endian_output() {
    let hexfile = run_checksum_hex(&[0x01, 0x02, 0x03, 0x04], "/CSR0:@append");
    let norm = hexfile.normalized_lossy();
    assert_eq!(
        norm.read_bytes_contiguous(0x1000, 6).unwrap(),
        vec![0x01, 0x02, 0x03, 0x04, 0x0A, 0x00]
    );
}

#[test]
fn test_cli_checksum_file_output() {
    let dir = temp_dir("cli_checksum_file");
    let input_path = dir.join("input.bin");
    let out_path = dir.join("csum.txt");
    write_file(&input_path, &[0x01, 0x02, 0x03, 0x04]);

    let args = vec![
        format!("/IN:{};0x1000", input_path.display()),
        format!("/CS0:{}", out_path.display()),
    ];

    let output = run_h3xy(&args);
    assert_success(&output);

    let text = std::fs::read_to_string(&out_path).unwrap();
    assert_eq!(text, "00,0A");
}

#[test]
fn test_cli_checksum_invalid_forced_pattern() {
    let dir = temp_dir("cli_checksum_bad");
    let input_path = dir.join("input.bin");
    write_file(&input_path, &[0x01, 0x02]);

    let args = vec![
        format!("/IN:{};0x1000", input_path.display()),
        "/CS0:@append;!0x1000-0x1001#F".to_string(),
    ];

    let output = run_h3xy(&args);
    assert!(!output.status.success());
}

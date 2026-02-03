use std::path::Path;

use h3xy::parse_intel_hex;

use crate::common::{assert_success, run_h3xy};

pub fn run_hex_output(args: Vec<String>, out_path: &Path) -> h3xy::HexFile {
    let output = run_h3xy(&args);
    assert_success(&output);
    let data = std::fs::read(out_path).unwrap();
    parse_intel_hex(&data).unwrap()
}

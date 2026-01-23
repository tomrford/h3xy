use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use h3xy::parse_intel_hex;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn temp_dir(prefix: &str) -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut dir = std::env::temp_dir();
    dir.push(format!("h3xy_{prefix}_{}_{}", std::process::id(), id));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn write_file(path: &Path, data: &[u8]) {
    std::fs::write(path, data).unwrap();
}

pub fn run_h3xy(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_h3xy"))
        .args(args)
        .output()
        .unwrap()
}

pub fn assert_success(output: &Output) {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("h3xy failed: {stderr}");
    }
}

pub fn run_hex_output(args: Vec<String>, out_path: &Path) -> h3xy::HexFile {
    let output = run_h3xy(&args);
    assert_success(&output);
    let data = std::fs::read(out_path).unwrap();
    parse_intel_hex(&data).unwrap()
}

pub fn read_nonempty_lines(path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).unwrap();
    text.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

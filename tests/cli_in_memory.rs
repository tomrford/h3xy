use std::collections::HashMap;

use h3xy::cli;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn test_execute_in_memory_merge_opaque_intel_hex() {
    let mut blocks = HashMap::new();
    let base_hex = h3xy::HexFile::with_segments(vec![h3xy::Segment::new(0x1000, vec![0x01])]);
    let merge_hex = h3xy::HexFile::with_segments(vec![h3xy::Segment::new(0x1000, vec![0xFF])]);
    blocks.insert("base".to_string(), base_hex);
    blocks.insert("merge".to_string(), merge_hex);

    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "h3xy_cli_mem_{stamp}_{}_{}",
        std::process::id(),
        count
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let out_path = dir.join("out.hex");
    let args = format!("base /MO:merge /XI -o {}", out_path.to_string_lossy());

    cli::execute_in_memory(&args, &blocks).unwrap();

    let output = std::fs::read(&out_path).unwrap();
    let hexfile = h3xy::parse_intel_hex(&output).unwrap();
    assert_eq!(hexfile.read_byte(0x1000), Some(0xFF));

    let _ = std::fs::remove_dir_all(dir);
}

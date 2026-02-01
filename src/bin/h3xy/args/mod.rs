//! HexView-compatible CLI argument parsing and execution.
//!
//! Processing order matches HexView (implemented subset):
//! 1. Read input file
//! 2. Open error log (/E)
//! 3. Set silent mode (/S)
//! 4. Import 16-bit Hex (/II2)
//! 5. Address mapping (/S08MAP, /S12MAP, /REMAP)
//! 6. dsPIC ops (/CDSPX, /CDSPS, /CDSPG)
//! 7. Fill ranges (/FR)
//! 8. Cut ranges (/CR)
//! 9. Merge files (/MT, /MO)
//! 10. Address range filter (/AR)
//! 11. Execute log commands (/L)
//! 12. Create single-region (/FA)
//! 13. Align (/AD, /AL)
//! 14. Split blocks (/SB)
//! 15. Swap bytes (/SWAPWORD, /SWAPLONG)
//! 16. Checksum (/CS)
//! 17. Export (/Xx)
//!
//! Note: /PB and /DP are not implemented (proprietary DLL-backed).

mod error;
mod execute;
mod ini;
mod io;
mod parse;
mod parse_util;
mod types;

use std::io::Write;
use std::process::ExitCode;
use std::{collections::HashMap, path::Path};

pub use error::{CliError, ExecuteOutput};
pub use types::Args;

pub fn run() -> ExitCode {
    let args = match Args::parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::FAILURE;
        }
    };

    if let Some(ref path) = args.error_log {
        let _ = std::fs::write(path, "");
    }

    if let Err(e) = args.execute() {
        if let Some(ref path) = args.error_log {
            let _ = std::fs::write(path, format!("{e}"));
        }
        if !args.silent {
            eprintln!("Error: {e}");
        }
        return ExitCode::FAILURE;
    }

    if args.write_version
        && let Some(ref path) = args.error_log
    {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path);
        if let Ok(ref mut file) = file {
            let _ = write!(file, "Hexview V{}", env!("CARGO_PKG_VERSION"));
        }
    }

    ExitCode::SUCCESS
}

pub fn execute_in_memory(
    args: &str,
    blocks: &HashMap<String, crate::HexFile>,
) -> Result<ExecuteOutput, CliError> {
    let parsed = Args::parse_from_str_with(args, |arg| {
        let path = Path::new(arg);
        arg.starts_with('/') && path.is_absolute() && blocks.contains_key(arg)
    })?;
    parsed.execute_with_blocks(blocks)
}

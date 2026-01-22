//! HexView-compatible CLI argument parsing and execution.
//!
//! Processing order matches HexView:
//! 1. Read input file
//! 2. Open error log (/E)
//! 3. Set silent mode (/S)
//! 4. Import 16-bit Hex (/II2)
//! 5. Address mapping (/s12map, /remap, etc.)
//! 6. Fill ranges (/FR)
//! 7. Cut ranges (/CR)
//! 8. Merge files (/MT, /MO)
//! 9. Address range filter (/AR)
//! 10. Execute log commands (/L)
//! 11. Create single-region (/FA)
//! 12. Postbuild (/PB)
//! 13. Align (/AD, /AL)
//! 14. Checksum (/CS)
//! 15. Data processing (/DP)
//! 16. Export (/Xx)

mod error;
mod execute;
mod ini;
mod io;
mod parse;
mod parse_util;
mod types;

use std::io::Write;
use std::process::ExitCode;

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

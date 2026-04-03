use std::process::ExitCode;

pub use h3xy::ops::{LogError, execute_log_commands, parse_log_commands};
pub use h3xy::*;

mod args;

fn main() -> ExitCode {
    args::run()
}

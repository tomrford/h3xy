use thiserror::Error;

use super::types::ParseArgError;

#[derive(Debug, Error)]
pub enum CliError {
    #[error(transparent)]
    Arg(#[from] ParseArgError),
    #[error(transparent)]
    Ops(#[from] h3xy::OpsError),
    #[error(transparent)]
    Log(#[from] h3xy::LogError),
    #[error(transparent)]
    Parse(#[from] h3xy::ParseError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Unsupported(String),
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ExecuteOutput {
    pub checksum_bytes: Option<Vec<u8>>,
}

use thiserror::Error;

use crate::io::ParseError;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Parse(#[from] ParseError),
}

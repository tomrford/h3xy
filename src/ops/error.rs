use thiserror::Error;

#[derive(Debug, Error)]
pub enum OpsError {
    #[error("address overflow during offset operation")]
    AddressOverflow,

    #[error("address {address:#X} not divisible by {divisor}")]
    AddressNotDivisible { address: u32, divisor: u32 },

    #[error("segment length {length} not a multiple of {expected} for {operation}")]
    LengthNotMultiple {
        length: usize,
        expected: usize,
        operation: String,
    },

    #[error("alignment must be a power of 2, got {0}")]
    InvalidAlignment(u32),
}

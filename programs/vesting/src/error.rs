use anchor_lang::prelude::*;

#[error_code]
pub enum VestingError {
    #[msg("Invalid arguments")]
    InvalidArguments,
    #[msg("Invalid claim before cliff")]
    InvalidClaimBeforeCliff,
    #[msg("Invalid claim amount")]
    InvalidClaimAmount,
    #[msg("Overflow")]
    Overflow,
    #[msg("Underflow")]
    Underflow,
}

use anchor_lang::prelude::*;

#[error_code]
pub enum Errors {
    #[msg("")]
    BadMetadata, // 6000

    #[msg("")]
    BadRuleset,

    #[msg("You're withdrawing more sol than was deposited!")]
    SolRentExemptViolation,
}

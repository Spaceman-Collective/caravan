use anchor_lang::prelude::*;

#[error_code]
pub enum Errors {
    #[msg("")]
    BadMetadata, // 6000

    #[msg("")]
    BadRuleset,

    #[msg("You're withdrawing more sol than was deposited!")]
    SolRentExemptViolation,

    #[msg("Trade doesn't have open spot to join")]
    TradeFull,

    #[msg("Error attaching vault to trade")]
    VaultAttachError,

    #[msg("Error locking vault")]
    VaultLockError,

    #[msg("Confirm Trade Error")]
    ConfirmTradeError,

    #[msg("Cancel trade error")]
    CancelTradeError,
}

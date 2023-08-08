use anchor_lang::prelude::*;

#[account]
pub struct Vault {
    pub creator: Pubkey,
    pub vault_id: u64,
    pub owner: Pubkey,
    pub is_frozen: bool,
}

impl GetSize for Vault {
    fn get_size() -> usize {
        return 32 + 8 + 32 + 1;
    }
}

#[account]
pub struct TradeAccount {
    pub trade_id: u64,
    pub creator_key: Pubkey,
    pub creator_vault_key: Option<Pubkey>,
    pub creator_confirmed: bool,
    pub acceptor_key: Option<Pubkey>,
    pub acceptor_vault_key: Option<Pubkey>,
    pub acceptor_confirmed: bool,
}

impl GetSize for TradeAccount {
    fn get_size() -> usize {
        return 8 + 32 + 32 + 32 + 32 + 3 + 2;
    }
}

pub trait GetSize {
    fn get_size() -> usize;
}

use anchor_lang::prelude::*;


#[account]
pub struct Vault {
  pub vault_id: u64,
  pub owner: Pubkey,
  pub is_frozen: bool,
}

impl GetSize for Vault {
  fn get_size() -> usize {
    return 8+32+1;
  }
}

#[account]
pub struct Trade {
  pub trade_id: u64,
  pub creator_vault_key: Pubkey,
  pub acceptor_vault_key: Pubkey,
}

impl GetSize for Trade {
  fn get_size() -> usize {
    return 8+32+32;
  }
}

pub trait GetSize {
  fn get_size() -> usize;
}
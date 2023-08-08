use anchor_lang::prelude::*;
use anchor_spl::{token::{Token, TokenAccount, Mint}, associated_token::AssociatedToken};
use spl_account_compression::{program::SplAccountCompression, Noop};
use mpl_bubblegum::state::TreeConfig;

use crate::{account::*, pnft::AuthorizationDataLocal};

#[derive(Accounts)]
#[instruction(vault_id:u64)]
pub struct CreateVault<'info> {
  #[account(mut)]
  pub payer: Signer<'info>,
  pub system_program: Program<'info, System>,

  #[account(
    init,
    payer=payer,
    seeds=[
      b"vault",
      payer.key().to_bytes().as_ref(),
      vault_id.to_be_bytes().as_ref(),
    ],
    bump,
    space=8+Vault::get_size()
  )]
  pub vault: Account<'info, Vault>
}

#[derive(Accounts)]
#[instruction(vault_id:u64)]
pub struct WithdrawNFT<'info> {
  pub payer: Signer<'info>,
  pub system_program: Program<'info, System>,
  /**
   * Only the Owner can Widraw NFTs from this Vault
   * Cannot withdraw nfts if the vault is frozen (currently being used in a Trade)
   */
  #[account(
    constraint = (vault.owner.key() == payer.key()) && (!vault.is_frozen)
  )]
  pub vault: Account<'info, Vault>,

  #[account(mut)]
  pub vault_ata: Account<'info, TokenAccount>,
  #[account(mut)]
  pub receiver_ata: Account<'info, TokenAccount>,
  pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(vault_id: u64, authorization_data: Option<AuthorizationDataLocal>, rules_acc_present: bool)]
pub struct WithdrawPNFT<'info> {
  pub payer: Signer<'info>,
  pub system_program: Program<'info, System>,
  /**
   * Only the Owner can Widraw NFTs from this Vault
   * Cannot withdraw nfts if the vault is frozen (currently being used in a Trade)
   */
  #[account(
    constraint = (vault.owner.key() == payer.key()) && (!vault.is_frozen),
    seeds=[
      b"vault",
      payer.key().to_bytes().as_ref(),
      vault_id.to_be_bytes().as_ref(),
    ],
    bump,
  )]
  pub vault: Account<'info, Vault>,

  #[account(mut)]
  pub vault_ata: Account<'info, TokenAccount>,
  #[account(
    mut, //will get initialized by mpl program
    associated_token::mint = nft_mint,
    associated_token::authority = receiver    
  )]
  pub receiver_ata: Account<'info, TokenAccount>,
  /// CHECK: The owner of the receiver ATA, passed into transfer() call
  pub receiver: UncheckedAccount<'info>,
  pub nft_mint: Account<'info, Mint>,
  #[account(
    mut,
    seeds=[
        mpl_token_metadata::state::PREFIX.as_bytes(),
        mpl_token_metadata::id().as_ref(),
        nft_mint.key().as_ref(),
    ],
    seeds::program = mpl_token_metadata::id(),
    bump
  )]
  pub metadata: UncheckedAccount<'info>,
  /// CHECK: Checked with seeds constraint
  #[account(
      seeds=[
          mpl_token_metadata::state::PREFIX.as_bytes(),
          mpl_token_metadata::id().as_ref(),
          nft_mint.key().as_ref(),
          mpl_token_metadata::state::EDITION.as_bytes(),
      ],
      seeds::program = mpl_token_metadata::id(),
      bump
  )]
  pub edition: UncheckedAccount<'info>,
  /// CHECK: seeds below
  #[account(mut,
    seeds=[
        mpl_token_metadata::state::PREFIX.as_bytes(),
        mpl_token_metadata::id().as_ref(),
        nft_mint.key().as_ref(),
        mpl_token_metadata::state::TOKEN_RECORD_SEED.as_bytes(),
        vault_ata.key().as_ref()
    ],
    seeds::program = mpl_token_metadata::id(),
    bump
  )]
  pub owner_token_record: UncheckedAccount<'info>,
  /// CHECK: seeds below
  #[account(mut,
          seeds=[
              mpl_token_metadata::state::PREFIX.as_bytes(),
              mpl_token_metadata::id().as_ref(),
              nft_mint.key().as_ref(),
              mpl_token_metadata::state::TOKEN_RECORD_SEED.as_bytes(),
              receiver_ata.key().as_ref()
          ],
          seeds::program = mpl_token_metadata::id(),
          bump
      )]
  pub receiver_token_record: UncheckedAccount<'info>,
  /// CHECK: Auth Rules Program
  #[account(address = mpl_token_auth_rules::id())]
  pub auth_rules_program: UncheckedAccount<'info>,
  /// CHECK: Deserialization errors, so we just manually check the ID
  #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
  pub sysvar_instructions: UncheckedAccount<'info>,
  pub token_program: Program<'info, Token>,
  pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
#[instruction(vault_id: u64, root: [u8; 32], data_hash: [u8; 32], creator_hash: [u8; 32], nonce: u64, index: u32)]
pub struct WithdrawCNFT<'info> {
  pub payer: Signer<'info>,
  pub system_program: Program<'info, System>,
  /**
   * Only the Owner can Widraw NFTs from this Vault
   * Cannot withdraw nfts if the vault is frozen (currently being used in a Trade)
   */
  #[account(
    constraint = (vault.owner.key() == payer.key()) && (!vault.is_frozen),
    seeds=[
      b"vault",
      payer.key().to_bytes().as_ref(),
      vault_id.to_be_bytes().as_ref(),
    ],
    bump,
  )]
  pub vault: Account<'info, Vault>,

  /// CHECK: Can be anything, could be a PDA
  pub receiver: UncheckedAccount<'info>,
  #[account(
    seeds = [merkle_tree.key().as_ref()],
    bump, 
    seeds::program = bubblegum_program.key()
  )]
  /// CHECK: This account is neither written to nor read from.
  pub tree_authority: Account<'info, TreeConfig>,
  /// CHECK: This account is neither written to nor read from.
  pub new_leaf_owner: UncheckedAccount<'info>, // receiver
  #[account(mut)]
  /// CHECK: This account is modified in the downstream program
  pub merkle_tree: UncheckedAccount<'info>,
  pub log_wrapper: Program<'info, Noop>,
  pub compression_program: Program<'info, SplAccountCompression>,
  /// CHECK: Checked via ID
  #[account(address = mpl_bubblegum::id())]
  pub bubblegum_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
#[instruction(vault_id: u64, amount: u64)]
pub struct WithdrawSPL<'info>{
  pub payer: Signer<'info>,
  pub system_program: Program<'info, System>,
  /**
   * Only the Owner can Widraw NFTs from this Vault
   * Cannot withdraw nfts if the vault is frozen (currently being used in a Trade)
   */
  #[account(
    constraint = (vault.owner.key() == payer.key()) && (!vault.is_frozen),
    seeds=[
      b"vault",
      payer.key().to_bytes().as_ref(),
      vault_id.to_be_bytes().as_ref(),
    ],
    bump,
  )]
  pub vault: Account<'info, Vault>,

  #[account(mut)]
  pub vault_ata: Account<'info, TokenAccount>,
  #[account(mut)]
  pub receiver_ata: Account<'info, TokenAccount>,
  pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(vault_id: u64, amount: u64)]
pub struct WithdrawSOL<'info>{
  pub payer: Signer<'info>,
  pub system_program: Program<'info, System>,
  /**
   * Only the Owner can Widraw NFTs from this Vault
   * Cannot withdraw nfts if the vault is frozen (currently being used in a Trade)
   */
  #[account(
    constraint = (vault.owner.key() == payer.key()) && (!vault.is_frozen),
    seeds=[
      b"vault",
      payer.key().to_bytes().as_ref(),
      vault_id.to_be_bytes().as_ref(),
    ],
    bump,
  )]
  pub vault: Account<'info, Vault>,

  #[account(mut)]
  pub receiver: AccountInfo<'info>,
  pub rent_sysvar: Sysvar<'info, Rent>
}

#[derive(Accounts)]
#[instruction(trade_id: u64)]
pub struct CreateTrade<'info>{
  #[account(mut)]
  pub payer: Signer<'info>,
  pub system_program: Program<'info, System>,

  #[account(
    init,
    payer=payer,
    seeds=[
      b"trade",
      payer.key().to_bytes().as_ref(),
      trade_id.to_be_bytes().as_ref(),
    ],
    bump,
    space=8+TradeAccount::get_size()
  )]
  pub trade: Account<'info, TradeAccount>
}

#[derive(Accounts)]
pub struct JoinTrade<'info>{
  pub payer: Signer<'info>,
  #[account(mut)]
  pub trade: Account<'info, TradeAccount>
}

#[derive(Accounts)]
pub struct AttachVault<'info>{
  pub payer: Signer<'info>,

  #[account(mut)]
  pub vault: Account<'info, Vault>,

  #[account(mut)]
  pub trade: Account<'info, TradeAccount>
}

#[derive(Accounts)]
pub struct LockTrade<'info> {
  pub payer: Signer<'info>,
  #[account(mut)]
  pub trade: Account<'info, TradeAccount>
}

#[derive(Accounts)]
pub struct CancelTrade<'info> {
  #[account(mut)]
  pub payer: Signer<'info>,
  #[account(mut)]
  pub trade: Account<'info, TradeAccount>,
  #[account(mut)]
  pub creator_vault: Option<Account<'info, Vault>>,
  #[account(mut)]
  pub acceptor_vault: Option<Account<'info, Vault>>,
}

#[derive(Accounts)]
pub struct ConfirmTrade<'info> {
  pub payer: Signer<'info>,
  #[account(mut)]
  pub trade: Account<'info, TradeAccount>,
  #[account(mut)]
  pub creator_vault: Account<'info, Vault>,
  #[account(mut)]
  pub acceptor_vault: Account<'info, Vault>,
}
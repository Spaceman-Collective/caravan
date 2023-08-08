use anchor_lang::prelude::*;
use anchor_spl::token::{transfer as transferSPL, Mint, Transfer as TransferNFT};
use mpl_token_metadata::{
    instruction::{builders::TransferBuilder, InstructionBuilder, TransferArgs},
    processor::AuthorizationData,
    state::{Metadata, ProgrammableConfig::V1, TokenMetadataAccount},
};

pub mod account;
pub mod context;
pub mod error;
pub mod pnft;

use crate::account::*;
use crate::pnft::*;
use crate::{context::*, error::Errors};

declare_id!("7V5r5RdjfBLQY1iZAHhqR1Y6nuL8RgT8ituGCyEviY2t");

#[program]
pub mod caravan {
    use anchor_lang::solana_program::{
        instruction::Instruction, program::invoke_signed, system_instruction::transfer,
    };

    use super::*;

    // Create Vault
    pub fn create_vault(ctx: Context<CreateVault>, vault_id: u64) -> Result<()> {
        ctx.accounts.vault.creator = ctx.accounts.payer.key();
        ctx.accounts.vault.vault_id = vault_id;
        ctx.accounts.vault.owner = ctx.accounts.payer.key();
        ctx.accounts.vault.is_frozen = false;
        Ok(())
    }

    // Deposits are done as standard TX
    // You can know what NFTs the Vault has by indexing it.

    // Withdraw NFT from Vault (Token Program)
    pub fn withdraw_nft(ctx: Context<WithdrawNFT>, vault_id: u64) -> Result<()> {
        let payer_account_bytes = ctx.accounts.payer.key().to_bytes();
        let vault_id_bytes = vault_id.to_be_bytes();
        let vault_seeds: &[&[u8]] = &[
            b"vault",
            payer_account_bytes.as_ref(),
            vault_id_bytes.as_ref(),
            &[*ctx.bumps.get("vault").unwrap()],
        ];
        let signers_seeds = &[vault_seeds];

        transferSPL(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferNFT {
                    from: ctx.accounts.vault_ata.to_account_info(),
                    to: ctx.accounts.receiver_ata.to_account_info(),
                    authority: ctx.accounts.vault.to_account_info(),
                },
                signers_seeds,
            ),
            1,
        )?;
        Ok(())
    }
    // Withdraw pNFT from Vault (MPL Token Program)
    pub fn withdraw_pnft<'info>(
        ctx: Context<'_, '_, '_, 'info, WithdrawPNFT<'info>>,
        vault_id: u64,
        authorization_data: Option<AuthorizationDataLocal>,
        rules_acc_present: bool,
    ) -> Result<()> {
        let rem_acc = &mut ctx.remaining_accounts.iter();
        let auth_rules = if rules_acc_present {
            Some(next_account_info(rem_acc)?)
        } else {
            None
        };

        let mut builder = TransferBuilder::new();

        builder
            .token(ctx.accounts.vault_ata.key())
            .token_owner(ctx.accounts.vault.key())
            .authority(ctx.accounts.vault.key())
            .destination(ctx.accounts.receiver_ata.key())
            .destination_owner(ctx.accounts.receiver.key())
            .mint(ctx.accounts.nft_mint.key())
            .metadata(ctx.accounts.metadata.key())
            .edition(ctx.accounts.edition.key())
            .payer(ctx.accounts.payer.key())
            .owner_token_record(ctx.accounts.owner_token_record.key())
            .destination_token_record(ctx.accounts.receiver_token_record.key());

        let mut account_infos = vec![
            //   0. `[writable]` Token account
            ctx.accounts.vault_ata.to_account_info(),
            //   1. `[]` Token account owner
            ctx.accounts.vault.to_account_info(),
            //   2. `[writable]` Destination token account
            ctx.accounts.receiver_ata.to_account_info(),
            //   3. `[]` Destination token account owner
            ctx.accounts.receiver.to_account_info(),
            //   4. `[]` Mint of token asset
            ctx.accounts.nft_mint.to_account_info(),
            //   5. `[writable]` Metadata account
            ctx.accounts.metadata.to_account_info(),
            //   6. `[optional]` Edition of token asset
            ctx.accounts.edition.to_account_info(),
            //   7. `[signer] Transfer authority (token or delegate owner)
            ctx.accounts.receiver.to_account_info(),
            //   8. `[optional, writable]` Owner record PDA
            ctx.accounts.owner_token_record.to_account_info(),
            //   9. `[optional, writable]` Destination record PDA
            ctx.accounts.receiver_token_record.to_account_info(),
            //   10. `[signer, writable]` Payer
            ctx.accounts.payer.to_account_info(),
            //   11. `[]` System Program
            ctx.accounts.system_program.to_account_info(),
            //   12. `[]` Instructions sysvar account
            ctx.accounts.sysvar_instructions.to_account_info(),
            //   13. `[]` SPL Token Program
            ctx.accounts.token_program.to_account_info(),
            //   14. `[]` SPL Associated Token Account program
            ctx.accounts.associated_token_program.to_account_info(),
            //   15. `[optional]` Token Authorization Rules Program
            ctx.accounts.auth_rules_program.to_account_info(),
            //   16. `[optional]` Token Authorization Rules account
            // ctx.accounts.auth_rules_pda.to_account_info(),
        ];

        let metadata = assert_decode_metadata(
            &ctx.accounts.nft_mint,
            &ctx.accounts.metadata.to_account_info(),
        )?;

        //if auth rules passed in, validate & include it in CPI call
        if let Some(config) = metadata.programmable_config {
            match config {
                V1 { rule_set } => {
                    if let Some(rule_set) = rule_set {
                        msg!("ruleset triggered");
                        //safe to unwrap here, it's expected
                        let rules_acc = auth_rules.unwrap();

                        //1. validate
                        require!(rule_set == *rules_acc.key, Errors::BadRuleset);

                        //2. add to builder
                        builder.authorization_rules_program(*ctx.accounts.auth_rules_program.key);
                        builder.authorization_rules(*rules_acc.key);

                        //3. add to accounts
                        account_infos.push(ctx.accounts.auth_rules_program.to_account_info());
                        account_infos.push(rules_acc.to_account_info());
                    }
                }
            }
        }

        let transfer_ix = builder
            .build(TransferArgs::V1 {
                amount: 1, //currently 1 only
                authorization_data: authorization_data.map(|authorization_data| {
                    AuthorizationData::try_from(authorization_data).unwrap()
                }),
            })
            .unwrap()
            .instruction();

        let payer_account_bytes = ctx.accounts.payer.key().to_bytes();
        let vault_id_bytes = vault_id.to_be_bytes();
        let vault_seeds: &[&[u8]] = &[
            b"vault",
            payer_account_bytes.as_ref(),
            vault_id_bytes.as_ref(),
            &[*ctx.bumps.get("vault").unwrap()],
        ];
        let signers_seeds = &[vault_seeds];

        invoke_signed(&transfer_ix, &account_infos, signers_seeds)?;

        Ok(())
    }

    // first 8 bytes of SHA256("global:transfer")
    const TRANSFER_DISCRIMINATOR: &'static [u8; 8] = &[163, 52, 200, 231, 140, 3, 69, 186];

    // Withdraw cNFT from Vault (MPL Bubblegum Program)
    pub fn withdraw_cnft<'info>(
        //Explicitly set the CTX lifetime to the function lifetime
        // This allows all the accountinfos to last til the end of the function call
        ctx: Context<'_, '_, '_, 'info, WithdrawCNFT<'info>>,
        vault_id: u64,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> Result<()> {
        let mut accounts: Vec<AccountMeta> = vec![
            AccountMeta::new_readonly(ctx.accounts.tree_authority.key(), false),
            AccountMeta::new_readonly(ctx.accounts.vault.key(), true),
            AccountMeta::new_readonly(ctx.accounts.vault.key(), false),
            AccountMeta::new_readonly(ctx.accounts.new_leaf_owner.key(), false),
            AccountMeta::new(ctx.accounts.merkle_tree.key(), false),
            AccountMeta::new_readonly(ctx.accounts.log_wrapper.key(), false),
            AccountMeta::new_readonly(ctx.accounts.compression_program.key(), false),
            AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
        ];

        let mut data: Vec<u8> = vec![];
        data.extend(TRANSFER_DISCRIMINATOR);
        data.extend(root);
        data.extend(data_hash);
        data.extend(creator_hash);
        data.extend(nonce.to_le_bytes());
        data.extend(index.to_le_bytes());

        let mut account_infos: Vec<AccountInfo> = vec![
            ctx.accounts.tree_authority.to_account_info(),
            ctx.accounts.vault.to_account_info(),
            ctx.accounts.vault.to_account_info(),
            ctx.accounts.new_leaf_owner.to_account_info(),
            ctx.accounts.merkle_tree.to_account_info(),
            ctx.accounts.log_wrapper.to_account_info(),
            ctx.accounts.compression_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ];

        for acc in ctx.remaining_accounts.iter() {
            accounts.push(AccountMeta::new_readonly(acc.key(), false));
            account_infos.push(acc.to_account_info());
        }

        let payer_account_bytes = ctx.accounts.payer.key().to_bytes();
        let vault_id_bytes = vault_id.to_be_bytes();
        let vault_seeds: &[&[u8]] = &[
            b"vault",
            payer_account_bytes.as_ref(),
            vault_id_bytes.as_ref(),
            &[*ctx.bumps.get("vault").unwrap()],
        ];
        let signers_seeds = &[vault_seeds];

        invoke_signed(
            &Instruction {
                program_id: ctx.accounts.bubblegum_program.key(),
                accounts,
                data,
            },
            account_infos.as_slice(),
            signers_seeds,
        )?;
        Ok(())
    }

    // Withdraw SPL Token
    pub fn withdraw_spl(ctx: Context<WithdrawSPL>, vault_id: u64, amount: u64) -> Result<()> {
        let payer_account_bytes = ctx.accounts.payer.key().to_bytes();
        let vault_id_bytes = vault_id.to_be_bytes();
        let vault_seeds: &[&[u8]] = &[
            b"vault",
            payer_account_bytes.as_ref(),
            vault_id_bytes.as_ref(),
            &[*ctx.bumps.get("vault").unwrap()],
        ];
        let signers_seeds = &[vault_seeds];

        transferSPL(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferNFT {
                    from: ctx.accounts.vault_ata.to_account_info(),
                    to: ctx.accounts.receiver_ata.to_account_info(),
                    authority: ctx.accounts.vault.to_account_info(),
                },
                signers_seeds,
            ),
            amount,
        )?;
        Ok(())
    }

    // Withdraw SOL
    pub fn withdraw_sol(ctx: Context<WithdrawSOL>, vault_id: u64, amount: u64) -> Result<()> {
        let rent = &Rent::from_account_info(&ctx.accounts.rent_sysvar.to_account_info())?;
        let rent_exempt_resverve = rent.minimum_balance(Vault::get_size());
        if ctx.accounts.vault.to_account_info().lamports() - amount < rent_exempt_resverve {
            return err!(Errors::SolRentExemptViolation);
        }

        let transfer_ix = transfer(
            &ctx.accounts.vault.key(),
            &ctx.accounts.receiver.key(),
            amount,
        );

        let payer_account_bytes = ctx.accounts.payer.key().to_bytes();
        let vault_id_bytes = vault_id.to_be_bytes();
        let vault_seeds: &[&[u8]] = &[
            b"vault",
            payer_account_bytes.as_ref(),
            vault_id_bytes.as_ref(),
            &[*ctx.bumps.get("vault").unwrap()],
        ];
        let signers_seeds = &[vault_seeds];

        invoke_signed(
            &transfer_ix,
            &[
                ctx.accounts.vault.to_account_info(),
                ctx.accounts.receiver.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            signers_seeds,
        )?;

        Ok(())
    }

    // Create Trade
    // Freezes Vault Withdraws
    pub fn create_trade(ctx: Context<CreateTrade>, trade_id: u64) -> Result<()> {
        ctx.accounts.trade.trade_id = trade_id;
        ctx.accounts.trade.creator_key = ctx.accounts.payer.key();
        ctx.accounts.trade.creator_vault_key = None;
        ctx.accounts.trade.acceptor_key = None;
        ctx.accounts.trade.acceptor_vault_key = None;
        ctx.accounts.trade.creator_confirmed = false;
        ctx.accounts.trade.acceptor_confirmed = false;
        Ok(())
    }

    pub fn join_trade(ctx: Context<JoinTrade>) -> Result<()> {
        if ctx.accounts.trade.acceptor_key == None {
            ctx.accounts.trade.acceptor_key = Some(ctx.accounts.payer.key())
        } else {
            return err!(Errors::TradeFull);
        }

        Ok(())
    }

    // Attach vault to trade
    pub fn attach_vault_to_trade(ctx: Context<AttachVault>) -> Result<()> {
        if ctx.accounts.trade.creator_key == ctx.accounts.payer.key()
            && !ctx.accounts.trade.creator_confirmed
        {
            // Can't change out vaults if you've confirmed
            ctx.accounts.trade.creator_vault_key = Some(ctx.accounts.vault.key());
            ctx.accounts.vault.is_frozen = true;
            // If you haven't confirmed and you change out vaults, the other person's confirmation gets reset
            ctx.accounts.trade.acceptor_confirmed = false;
        } else if ctx.accounts.trade.acceptor_key == Some(ctx.accounts.payer.key())
            && !ctx.accounts.trade.acceptor_confirmed
        {
            // Can't change out vaults if you've confirmed
            ctx.accounts.trade.acceptor_vault_key = Some(ctx.accounts.vault.key());
            ctx.accounts.vault.is_frozen = true;
            // If you haven't confirmed and you change out vaults, the other person's confirmation gets reset
            ctx.accounts.trade.creator_confirmed = false;
        } else {
            return err!(Errors::VaultAttachError);
        }
        Ok(())
    }

    // Lock trade
    pub fn lock_trade(ctx: Context<LockTrade>) -> Result<()> {
        if ctx.accounts.trade.creator_key == ctx.accounts.payer.key() {
            ctx.accounts.trade.creator_confirmed = true;
        } else if ctx.accounts.trade.acceptor_key == Some(ctx.accounts.payer.key()) {
            ctx.accounts.trade.acceptor_confirmed = true;
        } else {
            return err!(Errors::VaultLockError);
        }
        Ok(())
    }

    // Cancel Trade
    pub fn cancel_trade(ctx: Context<CancelTrade>) -> Result<()> {
        if ctx.accounts.trade.creator_key == ctx.accounts.payer.key() {
            // If the creator wants to cancel, close the whole trade account
            if ctx.accounts.trade.creator_vault_key.is_some()
                && ctx.accounts.creator_vault.as_mut().unwrap().owner == ctx.accounts.payer.key()
            {
                ctx.accounts.creator_vault.as_mut().unwrap().is_frozen = false;
            } else {
                return err!(Errors::CancelTradeError);
            }

            if ctx.accounts.trade.acceptor_vault_key.is_some()
                && ctx.accounts.acceptor_vault.as_mut().unwrap().owner
                    == ctx.accounts.trade.acceptor_key.unwrap().key()
            {
                ctx.accounts.acceptor_vault.as_mut().unwrap().is_frozen = false;
            } else {
                return err!(Errors::CancelTradeError);
            }

            ctx.accounts
                .trade
                .close(ctx.accounts.payer.to_account_info())?;
        } else if ctx.accounts.trade.acceptor_key == Some(ctx.accounts.payer.key()) {
            // If the counterparty wants to cancel, just reset the counterparty
            ctx.accounts.trade.acceptor_key = None;
            ctx.accounts.trade.acceptor_vault_key = None;
            ctx.accounts.trade.acceptor_confirmed = false;
            ctx.accounts.acceptor_vault.as_mut().unwrap().is_frozen = false;
        } else {
            return err!(Errors::CancelTradeError);
        }
        Ok(())
    }

    // Confirm Trade
    pub fn confirm_trade(ctx: Context<ConfirmTrade>) -> Result<()> {
        // Checks:
        //1. Confirm must be called by creator or acceptor
        //2. Creator and Acceptor must both have locked
        //3. Creator Vault and Accept vault must match those sent in

        if (ctx.accounts.trade.creator_key != ctx.accounts.payer.key()
            && ctx.accounts.trade.acceptor_key != Some(ctx.accounts.payer.key()))
            || (!ctx.accounts.trade.creator_confirmed || !ctx.accounts.trade.acceptor_confirmed)
            || (ctx.accounts.trade.creator_vault_key != Some(ctx.accounts.creator_vault.key()))
            || (ctx.accounts.trade.acceptor_vault_key != Some(ctx.accounts.acceptor_vault.key()))
        {
            return err!(Errors::ConfirmTradeError);
        }

        ctx.accounts.creator_vault.owner = ctx.accounts.trade.acceptor_key.unwrap().key();
        ctx.accounts.acceptor_vault.owner = ctx.accounts.trade.creator_key.key();
        Ok(())
    }
}

pub fn assert_decode_metadata<'info>(
    nft_mint: &Account<'info, Mint>,
    metadata_account: &AccountInfo<'info>,
) -> Result<Metadata> {
    let (key, _) = Pubkey::find_program_address(
        &[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            nft_mint.key().as_ref(),
        ],
        &mpl_token_metadata::id(),
    );
    if key != *metadata_account.key {
        require!(true, Errors::BadMetadata);
    }
    // Check account owner (redundant because of find_program_address above, but why not).
    if *metadata_account.owner != mpl_token_metadata::id() {
        return Err(error!(Errors::BadMetadata));
    }

    Ok(Metadata::from_account_info(metadata_account)?)
}

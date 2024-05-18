use crate::error::ErrorCode;
use crate::states::*;
use crate::utils::token::*;
use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use anchor_spl::{
    token::Token,
    token_interface::{Mint, TokenAccount},
};

#[derive(Accounts)]
pub struct DeployPair<'info> {
    /// The user performing the DeployPair
    pub payer: Signer<'info>,

    /// CHECK: create pool fee account
    #[account(
        mut,
        address = crate::create_pool_fee_receiver::id(),
    )]
    pub create_pool_fee: UncheckedAccount<'info>,

    /// CHECK: pool vault and lp mint authority
    #[account(
        seeds = [
            crate::AUTH_SEED.as_bytes(),
        ],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    /// The factory state to read protocol fees
    #[account(address = pool_state.load()?.amm_config)]
    pub amm_config: Box<Account<'info, AmmConfig>>,

    /// The program account of the pool in which the swap will be performed
    #[account(mut, constraint = pool_state.load()?.pool_creator == payer.key())]
    pub pool_state: AccountLoader<'info, PoolState>,

    /// The user token account for token_0
    #[account(
        mut,
    )]
    pub token_0_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: The user token account for token_1
    #[account(mut, address = payer.key())]
    pub token_1_account: UncheckedAccount<'info>,

    /// CHECK: The vault token account for token 0
    #[account(
        mut,
        constraint = token_0_vault.key() == pool_state.load()?.token_0_vault
    )]
    pub token_0_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: The vault token account for token 1
    #[account(
        mut,
        constraint = token_1_vault.key() == pool_state.load()?.token_1_vault && token_1_vault.get_lamports() >= MIN_AMOUNT_TO_DEPLOY
    )]
    pub token_1_vault: UncheckedAccount<'info>,

    /// The mint of token_0
    #[account(
        address = pool_state.load()?.token_0_mint 
    )]
    pub token_0_mint: Box<InterfaceAccount<'info, Mint>>,

    /// The program account for the most recent oracle observation
    #[account(mut, address = pool_state.load()?.observation_key)]
    pub observation_state: AccountLoader<'info, ObservationState>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,

    pub raydium_program: Program<'info, System>,
}

pub fn deploy_pair(ctx: Context<DeployPair>) -> Result<()> {
    let block_timestamp = solana_program::clock::Clock::get()?.unix_timestamp as u64;
    let pool_id = ctx.accounts.pool_state.key();
    let pool_state = &mut ctx.accounts.pool_state.load_mut()?;
    if !pool_state.get_status_by_bit(PoolStatusBitIndex::Deploy)
        || block_timestamp <= pool_state.open_time
    {
        return err!(ErrorCode::NotApproved);
    }

    let token_0_vault = ctx.accounts.token_0_vault.clone();
    let token_1_vault = ctx.accounts.token_1_vault.clone();

    // invoke Pyth program to get SOL price

    // check market cap

    // create Raydium CPMM pool with `FREEZED_AMOUNT` token_0 and `BALANCE_OF_DEPLOYED_POOL` token_1

    // burn the rest of token_0 in vault_0 
    token_burn(
        ctx.accounts.authority.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.token_0_mint.to_account_info(),
        ctx.accounts.token_0_vault.to_account_info(),
        ctx.accounts.token_0_vault.amount.checked_sub(FREEZED_AMOUNT).unwrap(),
        &[&[crate::AUTH_SEED.as_bytes(), &[pool_state.auth_bump]]],
    )?;

    // emit event

    // close oracle, vault_0, vault_1, authority account
    // & transfer the rest of balance vault_1 to pool creator

    Ok(())
}

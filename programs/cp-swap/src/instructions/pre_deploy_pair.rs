use std::borrow::Borrow;

use crate::error::ErrorCode;
use crate::states::*;
use crate::utils::{token::*, math::{to_decimals}, account::*};
use anchor_lang::prelude::*;
use anchor_spl::{
    token::Token,
    token_interface::{Mint, TokenAccount},
};
// use pyth_sdk_solana::state::SolanaPriceAccount;

#[derive(Accounts)]
pub struct PreDeployPair<'info> {
    /// The user performing the DeployPair
    #[account(address = crate::admin::id())]
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
    
    /// CHECK: The Pyth price feed account
    #[account(address = crate::sol_price_feed::id())]
    pub price_feed: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}

pub fn pre_deploy_pair(ctx: Context<PreDeployPair>) -> Result<()> {
    let observation_state = &mut ctx.accounts.observation_state.load_mut()?;
    let pool_state = &mut ctx.accounts.pool_state.load_mut()?;
    // @notice must lock the pool before deploy
    if pool_state.get_status_by_bit(PoolStatusBitIndex::Deploy)
    {
        return err!(ErrorCode::NotApproved);
    }
    // lock state to prevent any incoming actions
    pool_state.set_status(1);

    let frozen_amount = to_decimals(FROZEN_AMOUNT, ctx.accounts.token_0_mint.decimals.into());
    // let available_amount = to_decimals(AVAILABLE_AMOUNT, ctx.accounts.token_0_mint.decimals.into());
    let actual_token_0_amount = ctx.accounts.token_0_vault.amount.checked_sub(frozen_amount).unwrap();
    let actual_token_1_amount = ctx.accounts.token_1_vault.get_lamports().checked_add(BASE_INIT_TOKEN_1_AMOUNT).unwrap();

    // // invoke Pyth program to get SOL price
    // let token_1_price = {
    // let block_timestamp = solana_program::clock::Clock::get()?.unix_timestamp as u64;
    //     const STALENESS_THRESHOLD: u64 = 10; // staleness threshold in seconds
    //     let price_feed = SolanaPriceAccount::account_info_to_feed(&ctx.accounts.price_feed).unwrap();
    //     let current_price = price_feed
    //         .get_price_no_older_than(oracle::default_block_timestamp(), STALENESS_THRESHOLD)
    //         .unwrap();
    //     let display_price = from_decimals(
    //         u64::try_from(current_price.price).unwrap(),
    //         u32::try_from(-current_price.expo).unwrap()
    //     );
    //     u128::try_from(display_price).unwrap()
    // };

    // // check market cap
    // let token_0_price = observation_state.calculate_token_0_price(token_1_price);
    // let token_0_market_cap = {
    //     let amount_in_market =  available_amount.checked_sub(
    //         actual_token_0_amount
    //     )
    //     .unwrap();
    //     require_gt!(amount_in_market, 0, ErrorCode::InvalidMarketCap);
    //     token_0_price.checked_mul(amount_in_market as u128).unwrap()
    // };
    // require_gte!(token_0_market_cap, to_decimals(MIN_TOKEN_0_MARKET_CAP, ctx.accounts.token_0_mint.decimals.into()) as u128, ErrorCode::InvalidMarketCap);

    // // TODO: create Raydium CPMM pool with `FROZEN_AMOUNT` token_0 and `BALANCE_OF_DEPLOYED_POOL` token_1
    // @dev just test for now: transfer all balances to token_x_account
    transfer_token(
        ctx.accounts.authority.to_account_info(),
        ctx.accounts.token_0_account.to_account_info(),
        ctx.accounts.token_0_vault.to_account_info(),
        ctx.accounts.token_0_mint.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.token_0_vault.amount, // FROZEN_AMOUNT,
        ctx.accounts.token_0_mint.decimals,
        false,
        &[&[crate::AUTH_SEED.as_bytes(), &[pool_state.auth_bump]]],
    )?;
    transfer_native_token(
        ctx.accounts.token_1_vault.to_account_info(),
        ctx.accounts.token_1_account.to_account_info(),
        ctx.accounts.token_1_vault.get_lamports(),
        false,
        ctx.accounts.system_program.to_account_info(),
        &[&[
            POOL_VAULT_SEED.as_bytes(),
            ctx.accounts.pool_state.key().as_ref(),
            ctx.accounts.system_program.key().as_ref(),
            &[pool_state.vault_1_bump][..],
        ][..]],
    )?;

    // // burn the rest of token_0 in vault_0
    // token_burn(
    //     ctx.accounts.authority.to_account_info(),
    //     ctx.accounts.token_program.to_account_info(),
    //     ctx.accounts.token_0_mint.to_account_info(),
    //     ctx.accounts.token_0_vault.to_account_info(),
    //     actual_token_0_amount,
    //     &[&[crate::AUTH_SEED.as_bytes(), &[pool_state.auth_bump]]],
    // )?;

    // emit event
    let cumulative = observation_state.get_latest_cumulative();
    emit!(events::PreDeployPairEvent {
        pool_id: ctx.accounts.pool_state.key(),
        token_0_vault_before: actual_token_0_amount,
        token_1_vault_before: actual_token_1_amount,
        token_0_cumulative: cumulative.0,
        token_1_cumulative: cumulative.1,
    });


    // close `observation_state`, `vault_0`, `vault_1`, `pool_state` account
    // transfer the rest of balance of all accounts to `create_pool_fee``
    //
    // close token_0_vault token account
    close_token_account(
        ctx.accounts.authority.to_account_info().borrow(),
        ctx.accounts.token_0_vault.to_account_info().borrow(),
        ctx.accounts.create_pool_fee.to_account_info().borrow(),
        ctx.accounts.token_program.to_account_info().borrow(),
        &[&[crate::AUTH_SEED.as_bytes(), &[pool_state.auth_bump]]],
    )?;

    // close token_0_vault account
    close_account(
        ctx.accounts.token_0_vault.to_account_info().borrow(),
        ctx.accounts.create_pool_fee.to_account_info().borrow(),
    )?;

    // close token_1_vault
    close_account(
        ctx.accounts.token_1_vault.to_account_info().borrow(),
        ctx.accounts.create_pool_fee.to_account_info().borrow(),
    )?;

    // close pool_state
    close_account(
        ctx.accounts.pool_state.to_account_info().borrow(),
        ctx.accounts.create_pool_fee.to_account_info().borrow(),
    )?;

    // close observation
    close_account(
        ctx.accounts.observation_state.to_account_info().borrow(),
        ctx.accounts.create_pool_fee.to_account_info().borrow(),
    )?;

    Ok(())
}

use std::borrow::BorrowMut;
use crate::error::ErrorCode;
use crate::states::*;
use crate::utils::{close_account, close_token_account};
use crate::utils::{token::*, math::{to_decimals, from_decimals}};
use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use anchor_spl::{
    token::Token,
    token_interface::{Mint, TokenAccount},
};
use pyth_sdk_solana::state::SolanaPriceAccount;

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
    
    /// CHECK: The Pyth price feed account
    #[account(address = crate::sol_price_feed::id())]
    pub price_feed: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,

    pub raydium_program: Program<'info, System>,
}

pub fn deploy_pair(ctx: Context<DeployPair>) -> Result<()> {
    let block_timestamp = solana_program::clock::Clock::get()?.unix_timestamp as u64;
    let pool_state = &mut ctx.accounts.pool_state.load_mut()?;
    if !pool_state.get_status_by_bit(PoolStatusBitIndex::Deploy)
        || block_timestamp <= pool_state.open_time
    {
        return err!(ErrorCode::NotApproved);
    }

    // invoke Pyth program to get SOL price
    let token_1_price = {
        const STALENESS_THRESHOLD: u64 = 10; // staleness threshold in seconds
        let price_feed = SolanaPriceAccount::account_info_to_feed(&ctx.accounts.price_feed).unwrap();
        let current_price = price_feed
            .get_price_no_older_than(oracle::default_block_timestamp(), STALENESS_THRESHOLD)
            .unwrap();
        let display_price = from_decimals(
            u64::try_from(current_price.price).unwrap(),
            u32::try_from(-current_price.expo).unwrap()
        );
        u128::try_from(display_price).unwrap()
    };

    let freezed_amount = to_decimals(FREEZED_AMOUNT, ctx.accounts.token_0_mint.decimals.into());
    let available_amount = to_decimals(AVAILABLE_AMOUNT, ctx.accounts.token_0_mint.decimals.into());
    let actual_token_0_amount = ctx.accounts.token_0_vault.amount.checked_sub(freezed_amount).unwrap();
    let actual_token_1_amount = ctx.accounts.token_1_vault.get_lamports().checked_add(BASE_INIT_TOKEN_1_AMOUNT).unwrap();
    // check market cap
    let token_0_price = {
        let (token_0_price_x32, _) = pool_state.token_price_x32(
            actual_token_0_amount,
            actual_token_1_amount,
        );
        token_0_price_x32.checked_mul(token_1_price).unwrap()
    };
    let token_0_market_cap = {
        let amount_in_market =  available_amount.checked_sub(
            actual_token_0_amount
        )
        .unwrap();
        require_gt!(amount_in_market, 0, ErrorCode::InvalidMarketCap);
        token_0_price.checked_mul(amount_in_market as u128).unwrap()
    };
    require_gte!(token_0_market_cap, to_decimals(MIN_TOKEN_0_MARKET_CAP, ctx.accounts.token_0_mint.decimals.into()) as u128, ErrorCode::InvalidMarketCap);

    // create Raydium CPMM pool with `FREEZED_AMOUNT` token_0 and `BALANCE_OF_DEPLOYED_POOL` token_1

    // burn the rest of token_0 in vault_0 
    token_burn(
        ctx.accounts.authority.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.token_0_mint.to_account_info(),
        ctx.accounts.token_0_vault.to_account_info(),
        actual_token_0_amount,
        &[&[crate::AUTH_SEED.as_bytes(), &[pool_state.auth_bump]]],
    )?;

    // emit event
    emit!(events::DeployPairEvent {
        pool_id: ctx.accounts.pool_state.key(),
        token_0_vault_before: actual_token_0_amount,
        token_1_vault_before: actual_token_1_amount,
        token_0_market_cap: token_0_market_cap,
    });

    // close `observation_state`, `vault_0`, `vault_1`, `pool_state` account
    // transfer the rest of balance of all accounts to `create_pool_fee``

    // close token_0_vault
    close_token_account(
        ctx.accounts.authority.to_account_info().borrow_mut(),
        ctx.accounts.token_0_vault.to_account_info().borrow_mut(),
        ctx.accounts.create_pool_fee.to_account_info().borrow_mut(),
        ctx.accounts.token_program.to_account_info().borrow_mut(),
        &[&[crate::AUTH_SEED.as_bytes(), &[pool_state.auth_bump]]],
    )?;

    // close token_1_vault
    close_account(
        ctx.accounts.token_1_vault.to_account_info().borrow_mut(),
        ctx.accounts.create_pool_fee.to_account_info().borrow_mut(),
    )?;

    // close pool_state
    close_account(
        ctx.accounts.pool_state.to_account_info().borrow_mut(),
        ctx.accounts.create_pool_fee.to_account_info().borrow_mut(),
    )?;

    // close observation
    close_account(
        ctx.accounts.observation_state.to_account_info().borrow_mut(),
        ctx.accounts.create_pool_fee.to_account_info().borrow_mut(),
    )?;

    Ok(())
}

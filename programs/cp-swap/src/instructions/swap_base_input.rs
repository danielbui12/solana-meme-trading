use crate::curve::calculator::CurveCalculator;
use crate::curve::TradeDirection;
use crate::error::ErrorCode;
use crate::states::*;
use crate::utils::token::*;
use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use anchor_spl::{
    token::Token,
    token_interface::{Mint, TokenAccount}
};

#[derive(Accounts)]
pub struct Swap<'info> {
    /// The user performing the swap
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
    #[account(mut)]
    pub pool_state: AccountLoader<'info, PoolState>,

    /// The user token account for token_0
    #[account(mut)]
    pub token_0_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: The user token account for token_1
    #[account(mut)]
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
        constraint = token_1_vault.key() == pool_state.load()?.token_1_vault
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
}

/// Receive at least
/// Recommend for using when inputting token_1 on the UI 
pub fn swap_base_input(
    ctx: Context<Swap>,
    trade_direction: u8,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<()> {
    let is_zero_for_one = TradeDirection::ZeroForOne.compare_w_u8(trade_direction);
    let block_timestamp = solana_program::clock::Clock::get()?.unix_timestamp as u64;
    let pool_id = ctx.accounts.pool_state.key();
    let pool_state = &mut ctx.accounts.pool_state.load_mut()?;
    if !pool_state.get_status_by_bit(PoolStatusBitIndex::Swap)
        || block_timestamp <= pool_state.open_time
    {
        return err!(ErrorCode::NotApproved);
    }
    let token_0_vault = ctx.accounts.token_0_vault.clone();
    let token_1_vault = ctx.accounts.token_1_vault.clone();

    // Take transfer fees into account for actual amount transferred in
    require_gt!(amount_in, 0);

    // Calculate the trade amounts
    let (trade_fee_rate, total_input_token_amount, total_output_token_amount) = if is_zero_for_one {
        let (total_input_token_amount, total_output_token_amount) = pool_state
            .vault_amount_without_fee(
                token_0_vault.amount,
                token_1_vault.get_lamports(),
            );

        (
            ctx.accounts.amm_config.trade_from_zero_to_one_fee_rate,
            total_input_token_amount
                .checked_sub(FREEZED_AMOUNT)
                .unwrap(),
            total_output_token_amount
                .checked_add(BASE_INIT_TOKEN_1_AMOUNT)
                .unwrap(),
        )
    } else {
        let (total_output_token_amount, total_input_token_amount) = pool_state
            .vault_amount_without_fee(
                token_0_vault.amount,
                token_1_vault.get_lamports(),
            );

        (
            ctx.accounts.amm_config.trade_from_one_to_zero_fee_rate,
            total_input_token_amount
                .checked_add(BASE_INIT_TOKEN_1_AMOUNT)
                .unwrap(),
            total_output_token_amount
                .checked_sub(FREEZED_AMOUNT)
                .unwrap(),
        )
    };

    let constant_before = u128::from(total_input_token_amount)
        .checked_mul(u128::from(total_output_token_amount))
        .unwrap();

    let result = CurveCalculator::swap_base_input(
        u128::from(amount_in),
        u128::from(total_input_token_amount),
        u128::from(total_output_token_amount),
        trade_fee_rate,
        ctx.accounts.amm_config.protocol_fee_rate,
        ctx.accounts.amm_config.fund_fee_rate,
    )
    .ok_or(ErrorCode::ZeroTradingTokens)?;

    let constant_after = u128::from(result.new_swap_source_amount)
        .checked_mul(u128::from(result.new_swap_destination_amount))
        .unwrap();
    #[cfg(feature = "enable-log")]
    msg!(
        "source_amount_swapped:{}, destination_amount_swapped:{},constant_before:{},constant_after:{}",
        result.source_amount_swapped,
        result.destination_amount_swapped,
        constant_before,
        constant_after
    );
    require_gte!(constant_after, constant_before);
    require_eq!(
        u64::try_from(result.source_amount_swapped).unwrap(),
        amount_in
    );
    let protocol_fee = u64::try_from(result.protocol_fee).unwrap();
    let fund_fee = u64::try_from(result.fund_fee).unwrap();
    match TradeDirection::to_enum(trade_direction) {
        TradeDirection::ZeroForOne => {
            pool_state.protocol_fees_token_0 = pool_state
                .protocol_fees_token_0
                .checked_add(protocol_fee)
                .unwrap();
            pool_state.fund_fees_token_0 =
                pool_state.fund_fees_token_0.checked_add(fund_fee).unwrap();
        }
        TradeDirection::OneForZero => {
            pool_state.protocol_fees_token_1 = pool_state
                .protocol_fees_token_1
                .checked_add(protocol_fee)
                .unwrap();
            pool_state.fund_fees_token_1 =
                pool_state.fund_fees_token_1.checked_add(fund_fee).unwrap();
        }
    };

    let (actual_token_0_amount, actual_token_1_amount) = {
        let token_0_transfer_amount = amount_in;
        let token_1_transfer_amount = {
            let trade_fee = if is_zero_for_one { 
                result.padding_trade_fee
            } else { 0 };
            let amount_out = u64::try_from(
                result.destination_amount_swapped.checked_sub(trade_fee).unwrap()
            ).unwrap();
            require_gt!(amount_out, 0);
            require_gte!(
                amount_out,
                minimum_amount_out,
                ErrorCode::ExceededSlippage
            );
            amount_out
        };
        if is_zero_for_one {
            (token_0_transfer_amount, token_1_transfer_amount)
        } else {
            (token_1_transfer_amount, token_0_transfer_amount)
        }
    };
    let token_0_authority = if is_zero_for_one {
        ctx.accounts.payer.to_account_info()
    } else {
        ctx.accounts.authority.to_account_info()
    };
    transfer_token(
        token_0_authority,
        ctx.accounts.token_0_account.to_account_info(),
        ctx.accounts.token_0_vault.to_account_info(),
        ctx.accounts.token_0_mint.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        actual_token_0_amount,
        ctx.accounts.token_0_mint.decimals,
        is_zero_for_one.clone(),
        &[&[crate::AUTH_SEED.as_bytes(), &[pool_state.auth_bump]]],
    )?;

    transfer_native_token(
        ctx.accounts.token_1_vault.to_account_info(),
        ctx.accounts.token_1_account.to_account_info(),
        actual_token_1_amount,
        !is_zero_for_one.clone(),
        ctx.accounts.system_program.to_account_info(),
        &[&[crate::AUTH_SEED.as_bytes(), &[pool_state.auth_bump]]],
    )?;

    if !is_zero_for_one {
       // take the fee when swap from Native -> SPL token
        transfer_native_token(
            ctx.accounts.create_pool_fee.to_account_info(),
            ctx.accounts.token_1_account.to_account_info(),
            u64::try_from(result.margin_trade_fee).unwrap(),
            true,
            ctx.accounts.system_program.to_account_info(),
            &[],
        )?;  
    // } else {
    //     // take the fee when swap from SPL token -> Native
    //     transfer_native_token(
    //         ctx.accounts.token_1_vault.to_account_info(),
    //         ctx.accounts.create_pool_fee.to_account_info(),
    //         u64::try_from(result.padding_trade_fee).unwrap(),
    //         false,
    //         ctx.accounts.system_program.to_account_info(),
    //         &[&[crate::AUTH_SEED.as_bytes(), &[pool_state.auth_bump]]],
    //     )?;
    }

    emit!(SwapEvent {
        pool_id,
        input_vault_before: total_input_token_amount,
        output_vault_before: total_output_token_amount,
        input_amount: u64::try_from(result.source_amount_swapped).unwrap(),
        output_amount: u64::try_from(result.destination_amount_swapped).unwrap(),
        base_input: true,
        trade_direction,
    });

    // update observation oracle
    ctx.accounts.token_0_vault.reload()?;
    let (token_0_price_x64, token_1_price_x64) = if is_zero_for_one
    {
        pool_state.token_price_x32(
            ctx.accounts.token_0_vault.amount.checked_sub(FREEZED_AMOUNT).unwrap(),
            ctx.accounts.token_1_vault.get_lamports().checked_add(BASE_INIT_TOKEN_1_AMOUNT).unwrap(),
        )
    } else {
        pool_state.token_price_x32(
            ctx.accounts.token_1_vault.get_lamports().checked_add(BASE_INIT_TOKEN_1_AMOUNT).unwrap(),
            ctx.accounts.token_0_vault.amount.checked_sub(FREEZED_AMOUNT).unwrap(),
        )
    };

    ctx.accounts.observation_state.load_mut()?.update(
        oracle::block_timestamp(),
        token_0_price_x64,
        token_1_price_x64,
    );

    Ok(())
}

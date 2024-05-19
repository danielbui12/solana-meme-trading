use super::swap_base_input::Swap;
use crate::curve::{calculator::CurveCalculator, TradeDirection};
use crate::error::ErrorCode;
use crate::states::*;
use crate::utils::math::to_decimals;
use crate::utils::token::*;
use anchor_lang::prelude::*;
use anchor_lang::solana_program;

/// Pay at most
/// Recommend for using when inputting token_0 on the UI
pub fn swap_base_output(
    ctx: Context<Swap>,
    trade_direction: u8,
    max_amount_in: u64,
    amount_out_less_fee: u64,
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

    let freezed_amount = to_decimals(FREEZED_AMOUNT, ctx.accounts.token_0_mint.decimals.into());

    // Calculate the trade amounts
    let (trade_fee_rate, total_token_0_amount, total_token_1_amount) = if is_zero_for_one {
        let (total_token_0_amount, total_token_1_amount) =
            pool_state.vault_amount_without_fee(token_0_vault.amount, token_1_vault.get_lamports());

        (
            ctx.accounts.amm_config.trade_from_zero_to_one_fee_rate,
            total_token_0_amount.checked_sub(freezed_amount).unwrap(),
            total_token_1_amount
                .checked_add(BASE_INIT_TOKEN_1_AMOUNT)
                .unwrap(),
        )
    } else {
        let (total_token_1_amount, total_token_0_amount) =
            pool_state.vault_amount_without_fee(token_0_vault.amount, token_1_vault.get_lamports());

        (
            ctx.accounts.amm_config.trade_from_one_to_zero_fee_rate,
            total_token_0_amount
                .checked_add(BASE_INIT_TOKEN_1_AMOUNT)
                .unwrap(),
            total_token_1_amount.checked_sub(freezed_amount).unwrap(),
        )
    };

    let constant_before = u128::from(total_token_0_amount)
        .checked_mul(u128::from(total_token_1_amount))
        .unwrap();

    let result = CurveCalculator::swap_base_output(
        u128::from(amount_out_less_fee),
        u128::from(total_token_0_amount),
        u128::from(total_token_1_amount),
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

    // Re-calculate the source amount swapped based on what the curve says
    let (actual_token_0_amount, actual_token_1_amount) = {
        let token_0_transfer_amount = {
            let source_amount_swapped = u64::try_from(result.source_amount_swapped).unwrap();
            require_gt!(source_amount_swapped, 0);
            require_gte!(
                max_amount_in,
                source_amount_swapped,
                ErrorCode::ExceededSlippage
            );
            source_amount_swapped
        };
        require_eq!(
            u64::try_from(result.destination_amount_swapped).unwrap(),
            amount_out_less_fee
        );
        let token_1_transfer_amount = {
            let trade_fee = if is_zero_for_one {
                result.padding_trade_fee
            } else {
                0
            };
            amount_out_less_fee
                .checked_sub(u64::try_from(trade_fee).unwrap())
                .unwrap()
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
        &[&[
            POOL_VAULT_SEED.as_bytes(),
            ctx.accounts.pool_state.key().as_ref(),
            ctx.accounts.system_program.key().as_ref(),
            &[pool_state.vault_1_bump][..],
        ][..]],
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
    } else {
        // take the fee when swap from SPL token -> Native
        transfer_native_token(
            ctx.accounts.token_1_vault.to_account_info(),
            ctx.accounts.create_pool_fee.to_account_info(),
            u64::try_from(result.padding_trade_fee).unwrap(),
            false,
            ctx.accounts.system_program.to_account_info(),
            &[&[
                POOL_VAULT_SEED.as_bytes(),
                ctx.accounts.pool_state.key().as_ref(),
                ctx.accounts.system_program.key().as_ref(),
                &[pool_state.vault_1_bump][..],
            ][..]],
        )?;
    }

    emit!(SwapEvent {
        pool_id,
        token_0_vault_before: total_token_0_amount,
        token_1_vault_before: total_token_1_amount,
        input_amount: u64::try_from(result.source_amount_swapped).unwrap(),
        output_amount: u64::try_from(result.destination_amount_swapped).unwrap(),
        base_input: false,
        trade_direction,
    });

    ctx.accounts.token_0_vault.reload()?;
    let (token_0_price_x64, token_1_price_x64) = if is_zero_for_one {
        pool_state.token_price_x32(
            ctx.accounts
                .token_0_vault
                .amount
                .checked_sub(freezed_amount)
                .unwrap(),
            ctx.accounts
                .token_1_vault
                .get_lamports()
                .checked_add(BASE_INIT_TOKEN_1_AMOUNT)
                .unwrap(),
        )
    } else {
        pool_state.token_price_x32(
            ctx.accounts
                .token_1_vault
                .get_lamports()
                .checked_add(BASE_INIT_TOKEN_1_AMOUNT)
                .unwrap(),
            ctx.accounts
                .token_0_vault
                .amount
                .checked_sub(freezed_amount)
                .unwrap(),
        )
    };

    ctx.accounts.observation_state.load_mut()?.update(
        oracle::block_timestamp(),
        token_0_price_x64,
        token_1_price_x64,
    );

    Ok(())
}

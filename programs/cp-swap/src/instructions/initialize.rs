use std::ops::Deref;

use crate::curve::CurveCalculator;
use crate::error::ErrorCode;
use crate::states::*;
use crate::utils::*;
use anchor_lang::{accounts::interface_account::InterfaceAccount, prelude::*};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{mint_to, MintTo, Token},
    token_interface::Mint,
};
use spl_memo::solana_program::program_pack::Pack;

#[derive(Accounts)]
pub struct Initialize<'info> {
    /// Address paying to create the pool. Can be anyone
    #[account(mut)]
    pub creator: Signer<'info>,

    /// Which config the pool belongs to.
    pub amm_config: Box<Account<'info, AmmConfig>>,

    /// CHECK: pool vault and token mint authority
    #[account(
        seeds = [
            crate::AUTH_SEED.as_bytes(),
        ],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    /// Initialize an account to store the pool state
    #[account(
        init,
        seeds = [
            POOL_SEED.as_bytes(),
            amm_config.key().as_ref(),
            token_0_mint.key().as_ref(),
            // token_1_mint.key().as_ref(),
        ],
        bump,
        payer = creator,
        space = PoolState::LEN
    )]
    pub pool_state: AccountLoader<'info, PoolState>,

    /// Token_0 mint, the key must smaller then token_1 mint.
    #[account(
        mut,
        seeds = [
            crate::CREATE_MINT_SEED.as_bytes(),
        ],
        bump,
        mint::token_program = token_program,
        mint::authority = token_0_mint,
        constraint = token_0_mint.supply == 0 @ ErrorCode::IncorrectToken0Mint,
        // constraint = token_0_mint.key() < token_1_mint.key(),
    )]
    pub token_0_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: Token_0 vault for the pool
    #[account(
        mut,
        seeds = [
            POOL_VAULT_SEED.as_bytes(),
            pool_state.key().as_ref(),
            token_0_mint.key().as_ref()
        ],
        bump,
    )]
    pub token_0_vault: UncheckedAccount<'info>,

    /// CHECK: Token_1 vault for the pool
    #[account(
        mut,
        seeds = [
            POOL_VAULT_SEED.as_bytes(),
            pool_state.key().as_ref(),
            system_program.key().as_ref(),  
        ],
        bump,
    )]
    pub token_1_vault: UncheckedAccount<'info>,

    /// CHECK: create pool fee account
    #[account(
        mut,
        address = crate::create_pool_fee_receiver::id(),
    )]
    pub create_pool_fee: UncheckedAccount<'info>,

    /// an account to store oracle observations
    #[account(
        init,
        seeds = [
            OBSERVATION_SEED.as_bytes(),
            pool_state.key().as_ref(),
        ],
        bump,
        payer = creator,
        space = ObservationState::LEN
    )]
    pub observation_state: AccountLoader<'info, ObservationState>,
    /// Program to create mint account and mint tokens
    pub token_program: Program<'info, Token>,
    /// Program to create an ATA for receiving position NFT
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// To create a new program account
    pub system_program: Program<'info, System>,
    /// Sysvar for program account
    pub rent: Sysvar<'info, Rent>,
}

pub fn initialize(ctx: Context<Initialize>, open_time: u64) -> Result<()> {
    if !is_supported_mint(&ctx.accounts.token_0_mint).unwrap() {
        return err!(ErrorCode::NotSupportMint);
    }

    if ctx.accounts.amm_config.disable_create_pool {
        return err!(ErrorCode::NotApproved);
    }

    // due to stack/heap limitations, we have to create redundant new accounts ourselves.
    create_token_account(
        &ctx.accounts.authority.to_account_info(),
        &ctx.accounts.creator.to_account_info(),
        &ctx.accounts.token_0_vault.to_account_info(),
        &ctx.accounts.token_0_mint.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        &ctx.accounts.token_program.to_account_info(),
        &[&[
            POOL_VAULT_SEED.as_bytes(),
            ctx.accounts.pool_state.key().as_ref(),
            ctx.accounts.token_0_mint.key().as_ref(),
            &[ctx.bumps.token_0_vault][..],
        ][..]],
    )?;

    let total_supply = to_decimals(FREEZED_AMOUNT, ctx.accounts.token_0_mint.decimals.into()) 
        + to_decimals(AVAILABLE_AMOUNT, ctx.accounts.token_0_mint.decimals.into());
    mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                authority: ctx.accounts.token_0_mint.to_account_info(),
                to: ctx.accounts.token_0_vault.to_account_info(),
                mint: ctx.accounts.token_0_mint.to_account_info(),
            },
            &[&[
              crate::CREATE_MINT_SEED.as_bytes(),
              &[ctx.bumps.token_0_mint],
            ][..]],
        ),
        total_supply,
    )?;

    let mut observation_state = ctx.accounts.observation_state.load_init()?;
    observation_state.pool_id = ctx.accounts.pool_state.key();

    let pool_state = &mut ctx.accounts.pool_state.load_init()?;
   
    let token_0_vault =
        spl_token::state::Account::unpack(
            ctx.accounts
                .token_0_vault
                .to_account_info()
                .try_borrow_data()?
                .deref())?;
    let token_1_vault = ctx
        .accounts
        .token_1_vault
        .to_account_info()
        .get_lamports()
        .checked_add(BASE_INIT_TOKEN_1_AMOUNT)
        .unwrap();

    CurveCalculator::validate_supply(token_0_vault.amount)?;

    let liquidity = U128::from(token_0_vault.amount)
        .checked_mul(token_1_vault.into())
        .unwrap()
        .integer_sqrt()
        .as_u64();

    // Charge the fee to create a pool
    if ctx.accounts.amm_config.create_pool_fee != 0 {
        transfer_native_token(
            ctx.accounts.create_pool_fee.to_account_info(),
            ctx.accounts.creator.to_account_info(),
            u64::from(ctx.accounts.amm_config.create_pool_fee),
            true,
            ctx.accounts.system_program.to_account_info(),
            &[],
        )?;
    }

    pool_state.initialize(
        ctx.bumps.authority,
        ctx.bumps.token_1_vault,
        liquidity,
        open_time,
        ctx.accounts.creator.key(),
        ctx.accounts.amm_config.key(),
        ctx.accounts.token_0_vault.key(),
        ctx.accounts.token_1_vault.key(),
        &ctx.accounts.token_0_mint,
        // &ctx.accounts.token_1_mint,
        ctx.accounts.observation_state.key(),
    );

    Ok(())
}

pub mod curve;
pub mod error;
pub mod instructions;
pub mod states;
pub mod utils;

use crate::curve::fees::FEE_RATE_DENOMINATOR_VALUE;
use anchor_lang::prelude::*;
use instructions::*;

use solana_security_txt::security_txt;

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    name: "booster-swap",
    project_url: "https://github.com/danielbui12/booster-swap",
    contacts: "link:huytung139@gmail.com",
    policy: "https://github.com/danielbui12/booster-swap",
    source_code: "https://github.com/danielbui12/booster-swap",
    preferred_languages: "en",
    auditors: "#"
}

#[cfg(feature = "devnet")]
declare_id!("HdNeVJt9x8p5G5Q99A3PySR4bNnzaLzHdSAw5B5eWZzC");
#[cfg(not(feature = "devnet"))]
declare_id!("HdNeVJt9x8p5G5Q99A3PySR4bNnzaLzHdSAw5B5eWZzC");

pub mod admin {
    use anchor_lang::prelude::declare_id;
    #[cfg(feature = "devnet")]
    declare_id!("EgwWVewxT4qrvkSpfx3T6hMUztGZPR8XiAGRiYGKdUc7");
    #[cfg(not(feature = "devnet"))]
    declare_id!("EgwWVewxT4qrvkSpfx3T6hMUztGZPR8XiAGRiYGKdUc7");
}

pub mod create_pool_fee_receiver {
    use anchor_lang::prelude::declare_id;
    #[cfg(feature = "devnet")]
    declare_id!("Kd8e8t428wuB68bpksHTqu4VbM97cqYa3AKP3osYsKH");
    #[cfg(not(feature = "devnet"))]
    declare_id!("Kd8e8t428wuB68bpksHTqu4VbM97cqYa3AKP3osYsKH");
}

pub mod sol_price_feed {
    use anchor_lang::prelude::declare_id;
    #[cfg(feature = "devnet")]
    declare_id!("J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix");
    #[cfg(not(feature = "devnet"))]
    declare_id!("H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG");
}

pub const AUTH_SEED: &str = "vault_auth_seed";
pub const CREATE_MINT_SEED: &str = "create_mint";

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct MintParams {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub decimals: u8,
}

#[program]
pub mod booster_swap {
    use super::*;

    // The configuration of AMM protocol, include trade fee and protocol fee
    /// # Arguments
    ///
    /// * `ctx`- The accounts needed by instruction.
    /// * `index` - The index of amm config, there may be multiple config.
    /// * `trade_from_zero_to_one_fee_rate` - Trade token_0 -> token_1 fee rate, can be changed.
    /// * `trade_from_one_to_zero_fee_rate` - Trade token_1 -> token_0 fee rate, can be changed.
    /// * `fund_fee_rate` - Fund fee rate, can be changed.
    /// * `create_pool_fee` - Create pool fee rate, can be changed.
    /// * `required_amount_0_in` - The required initial amount of token_0 in liquidity pool
    /// * `basic_amount_1_in` the initial virtual amount of token_1 in liquidity pool
    ///
    pub fn create_amm_config(
        ctx: Context<CreateAmmConfig>,
        index: u16,
        trade_from_zero_to_one_fee_rate: u64,
        trade_from_one_to_zero_fee_rate: u64,
        protocol_fee_rate: u64,
        fund_fee_rate: u64,
        create_pool_fee: u64,
    ) -> Result<()> {
        assert!(trade_from_zero_to_one_fee_rate < FEE_RATE_DENOMINATOR_VALUE);
        assert!(trade_from_one_to_zero_fee_rate <= FEE_RATE_DENOMINATOR_VALUE);
        instructions::create_amm_config(
            ctx,
            index,
            trade_from_zero_to_one_fee_rate,
            trade_from_one_to_zero_fee_rate,
            protocol_fee_rate,
            fund_fee_rate,
            create_pool_fee,
        )
    }

    /// Creates a new mint
    ///
    /// # Arguments
    ///
    /// * `ctx`- The context of accounts
    /// * `metadata`- The metadata of mint
    ///
    pub fn create_mint(ctx: Context<CreateMint>, metadata: MintParams) -> Result<()> {
        instructions::create_mint(ctx, metadata)
    }

    /// TEST ONLY
    pub fn mint_tokens(ctx: Context<MintTokens>, quantity: u64) -> Result<()> {
        instructions::mint_tokens(ctx, quantity)
    }

    /// Updates the owner of the amm config
    /// Must be called by the current owner or admin
    ///
    /// # Arguments
    ///
    /// * `ctx`- The context of accounts
    /// * `trade_fee_rate`- The new trade fee rate of amm config, be set when `param` is 0
    /// * `protocol_fee_rate`- The new protocol fee rate of amm config, be set when `param` is 1
    /// * `fund_fee_rate`- The new fund fee rate of amm config, be set when `param` is 2
    /// * `new_owner`- The config's new owner, be set when `param` is 3
    /// * `new_fund_owner`- The config's new fund owner, be set when `param` is 4
    /// * `param`- The value can be 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9, otherwise will report a error
    ///
    pub fn update_amm_config(ctx: Context<UpdateAmmConfig>, param: u8, value: u64) -> Result<()> {
        instructions::update_amm_config(ctx, param, value)
    }

    /// Update pool status for given value
    ///
    /// # Arguments
    ///
    /// * `ctx`- The context of accounts
    /// * `status` - The value of status
    ///
    pub fn update_pool_status(ctx: Context<UpdatePoolStatus>, status: u8) -> Result<()> {
        instructions::update_pool_status(ctx, status)
    }

    /// Collect the protocol fee accrued to the pool
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context of accounts
    /// * `amount_0_requested` - The maximum amount of token_0 to send, can be 0 to collect fees in only token_1
    /// * `amount_1_requested` - The maximum amount of token_1 to send, can be 0 to collect fees in only token_0
    ///
    pub fn collect_protocol_fee(
        ctx: Context<CollectProtocolFee>,
        amount_0_requested: u64,
        amount_1_requested: u64,
    ) -> Result<()> {
        instructions::collect_protocol_fee(ctx, amount_0_requested, amount_1_requested)
    }

    /// Collect the fund fee accrued to the pool
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context of accounts
    /// * `amount_0_requested` - The maximum amount of token_0 to send, can be 0 to collect fees in only token_1
    /// * `amount_1_requested` - The maximum amount of token_1 to send, can be 0 to collect fees in only token_0
    ///
    pub fn collect_fund_fee(
        ctx: Context<CollectFundFee>,
        amount_0_requested: u64,
        amount_1_requested: u64,
    ) -> Result<()> {
        instructions::collect_fund_fee(ctx, amount_0_requested, amount_1_requested)
    }

    /// Creates a pool for the given token pair and the initial price
    ///
    /// # Arguments
    ///
    /// * `ctx`- The context of accounts
    /// * `init_amount_0` - the initial amount_0 to deposit
    /// * `init_amount_1` - the initial amount_1 to deposit
    /// * `open_time` - the timestamp allowed for swap
    ///
    pub fn initialize(ctx: Context<Initialize>, open_time: u64) -> Result<()> {
        instructions::initialize(ctx, open_time)
    }

    // /// Withdraw token from Booster CPMM
    // ///
    // /// # Arguments
    // ///
    // /// * `ctx`- The context of accounts
    // /// * `lp_token_amount` - Amount of pool tokens to burn. User receives an output of token a and b based on the percentage of the pool tokens that are returned.
    // /// * `minimum_token_0_amount` -  Minimum amount of token 0 to receive, prevents excessive slippage
    // /// * `minimum_token_1_amount` -  Minimum amount of token 1 to receive, prevents excessive slippage
    // ///
    // pub fn withdraw(
    //     ctx: Context<Withdraw>,
    //     lp_token_amount: u64,
    //     minimum_token_0_amount: u64,
    //     minimum_token_1_amount: u64,
    // ) -> Result<()> {
    //     instructions::withdraw(
    //         ctx,
    //         lp_token_amount,
    //         minimum_token_0_amount,
    //         minimum_token_1_amount,
    //     )
    // }

    /// Swap the tokens in the pool base input amount
    ///
    /// # Arguments
    ///
    /// * `ctx`- The context of accounts
    /// * `amount_in` -  input amount to transfer, output to DESTINATION is based on the exchange rate
    /// * `minimum_amount_out` -  Minimum amount of output token, prevents excessive slippage
    ///
    pub fn swap_base_input(
        ctx: Context<Swap>,
        trade_direction: u8,
        amount_in: u64,
        minimum_amount_out: u64,
    ) -> Result<()> {
        instructions::swap_base_input(ctx, trade_direction, amount_in, minimum_amount_out)
    }

    /// Swap the tokens in the pool base output amount
    ///
    /// # Arguments
    ///
    /// * `ctx`- The context of accounts
    /// * `max_amount_in` -  input amount prevents excessive slippage
    /// * `amount_out` -  amount of output token
    ///
    pub fn swap_base_output(
        ctx: Context<Swap>,
        trade_direction: u8,
        max_amount_in: u64,
        amount_out: u64,
    ) -> Result<()> {
        instructions::swap_base_output(ctx, trade_direction, max_amount_in, amount_out)
    }

    /// Deploy pair
    ///
    /// # Arguments
    ///
    /// * `ctx`- The context of accounts
    ///
    pub fn deploy_pair(ctx: Context<DeployPair>) -> Result<()> {
        instructions::deploy_pair(ctx)
    }
}

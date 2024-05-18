use anchor_lang::prelude::*;

pub const AMM_CONFIG_SEED: &str = "amm_config";

/// Holds the current owner of the factory
#[account]
#[derive(Default, Debug)]
pub struct AmmConfig {
    /// Bump to identify PDA
    pub bump: u8,
    /// Status to control if new pool can be create
    pub disable_create_pool: bool,
    /// Config index
    pub index: u16,
    /// The trade token_0 -> token_1 fee, denominated in hundredths of a bip (10^-6)
    pub trade_from_zero_to_one_fee_rate: u64,
    /// The trade token_1 -> token_0 fee, denominated in hundredths of a bip (10^-6)
    pub trade_from_one_to_zero_fee_rate: u64,
    /// The protocol fee
    pub protocol_fee_rate: u64,
    /// The fund fee, denominated in hundredths of a bip (10^-6)
    pub fund_fee_rate: u64,
    /// Fee for create a new pool
    pub create_pool_fee: u64,
    /// Address of the protocol fee owner
    pub protocol_owner: Pubkey,
    /// Address of the fund fee owner
    pub fund_owner: Pubkey,
    /// padding
    pub padding: [u64; 16],
}

impl AmmConfig {
    pub const LEN: usize = 8 // discriminator 
      + 1 // u8
      + 1 // bool
      + 2 // u16
      + 8 * 5 // u64
      + 32 * 2 // Pubkey
      + 8 * 16 // u64
      ;
}

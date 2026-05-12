use anchor_lang::prelude::*;

use crate::error::SaleError;
use crate::seeds::{LISTING_STATE_SEED, PROGRAM_CONFIG_SEED};
use crate::state::{ListingState, ProgramConfig};

#[derive(Accounts)]
#[instruction(asset_id: Pubkey, seller_wallet: Pubkey)]
pub struct HousekeepingClear<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [PROGRAM_CONFIG_SEED],
        bump = program_config.bump,
        has_one = admin @ SaleError::NotAdmin,
    )]
    pub program_config: Account<'info, ProgramConfig>,

    #[account(
        mut,
        seeds = [LISTING_STATE_SEED, asset_id.as_ref(), seller_wallet.as_ref()],
        bump = listing_state.bump,
    )]
    pub listing_state: Account<'info, ListingState>,
}

pub fn handler(
    ctx: Context<HousekeepingClear>,
    _asset_id: Pubkey,
    _seller_wallet: Pubkey,
) -> Result<()> {
    let s = &mut ctx.accounts.listing_state;
    s.active_nonce = None;
    Ok(())
}

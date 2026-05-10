use anchor_lang::prelude::*;

use crate::error::SaleError;
use crate::seeds::LISTING_STATE_SEED;
use crate::state::ListingState;

// Read from build env so tests can override:
//   ADMIN_WALLET="<base58 pubkey>" anchor build
// In production, set this in CI before the prod build.
pub fn admin_wallet() -> Pubkey {
    let s = option_env!("ADMIN_WALLET")
        .unwrap_or("11111111111111111111111111111111"); // System program == "no admin set"
    s.parse().expect("ADMIN_WALLET env must be a valid base58 pubkey")
}

#[derive(Accounts)]
#[instruction(asset_id: Pubkey, seller_wallet: Pubkey)]
pub struct HousekeepingClear<'info> {
    pub admin: Signer<'info>,

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
    require_keys_eq!(ctx.accounts.admin.key(), admin_wallet(), SaleError::NotAdmin);
    let s = &mut ctx.accounts.listing_state;
    s.active_nonce = None;
    Ok(())
}

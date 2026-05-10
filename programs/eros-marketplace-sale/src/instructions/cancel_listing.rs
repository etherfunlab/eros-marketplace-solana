use anchor_lang::prelude::*;

use crate::error::SaleError;
use crate::seeds::LISTING_STATE_SEED;
use crate::state::ListingState;

#[derive(Accounts)]
pub struct CancelListing<'info> {
    /// The seller signs to cancel their own listing.
    pub seller: Signer<'info>,

    #[account(
        mut,
        seeds = [LISTING_STATE_SEED, listing_state.asset_id.as_ref(), seller.key().as_ref()],
        bump = listing_state.bump,
        constraint = listing_state.seller_wallet == seller.key() @ SaleError::RegistryAssetMismatch,
    )]
    pub listing_state: Account<'info, ListingState>,
}

pub fn handler(ctx: Context<CancelListing>) -> Result<()> {
    let s = &mut ctx.accounts.listing_state;
    require!(s.active_nonce.is_some(), SaleError::NoActiveListing);
    s.active_nonce = None;
    Ok(())
}

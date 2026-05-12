use anchor_lang::prelude::*;

use crate::error::SaleError;
use crate::seeds::{LISTING_STATE_SEED, PROGRAM_CONFIG_SEED};
use crate::state::{ListingState, ProgramConfig};

#[derive(Accounts)]
#[instruction(asset_id: Pubkey, seller_wallet: Pubkey, listing_nonce: u64)]
pub struct SetListingQuote<'info> {
    /// Pays rent on first listing for this (asset, seller). Typically the
    /// marketplace-svc service wallet, NOT the seller. Seller-supplied
    /// signature on the SaleOrder is verified separately at execute_purchase
    /// time, not here.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Authorized admin. Required so an attacker cannot advance any seller's
    /// listing nonce to a high value (DoS) or activate a known signed order
    /// nonce — the bug fixed here.
    pub admin: Signer<'info>,

    #[account(
        seeds = [PROGRAM_CONFIG_SEED],
        bump = program_config.bump,
        has_one = admin @ SaleError::NotAdmin,
    )]
    pub program_config: Account<'info, ProgramConfig>,

    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + ListingState::INIT_SPACE,
        seeds = [LISTING_STATE_SEED, asset_id.as_ref(), seller_wallet.as_ref()],
        bump,
    )]
    pub listing_state: Account<'info, ListingState>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<SetListingQuote>,
    asset_id: Pubkey,
    seller_wallet: Pubkey,
    listing_nonce: u64,
) -> Result<()> {
    let s = &mut ctx.accounts.listing_state;

    // First time? Initialize the immutable identity fields. Subsequent calls
    // overwrite active_nonce + last_seen_nonce only.
    if s.last_seen_nonce == 0 && s.active_nonce.is_none() && s.asset_id == Pubkey::default() {
        s.asset_id = asset_id;
        s.seller_wallet = seller_wallet;
        s.bump = ctx.bumps.listing_state;
    } else {
        // Identity must be stable for this PDA (the seeds enforce this, but
        // belt-and-braces).
        require_keys_eq!(s.asset_id, asset_id, SaleError::RegistryAssetMismatch);
        require_keys_eq!(s.seller_wallet, seller_wallet, SaleError::RegistryAssetMismatch);
    }

    require!(
        listing_nonce > s.last_seen_nonce,
        SaleError::NonceNotMonotonic
    );

    s.active_nonce = Some(listing_nonce);
    s.last_seen_nonce = listing_nonce;

    Ok(())
}

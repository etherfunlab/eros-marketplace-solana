use anchor_lang::prelude::*;

use crate::error::SaleError;
use crate::seeds::{MANIFEST_REGISTRY_SEED, ROYALTY_REGISTRY_SEED};
use crate::state::{ManifestRegistry, RoyaltyRegistry};

#[derive(Accounts)]
#[instruction(asset_id: Pubkey)]
pub struct InitRegistries<'info> {
    /// Caller pays rent for both PDAs. In practice this is the marketplace-svc
    /// service wallet at mint time.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Immutable royalty + platform fee registry. `init` (not `init_if_needed`)
    /// — re-init must fail.
    #[account(
        init,
        payer = payer,
        space = 8 + RoyaltyRegistry::INIT_SPACE,
        seeds = [ROYALTY_REGISTRY_SEED, asset_id.as_ref()],
        bump,
    )]
    pub royalty_registry: Account<'info, RoyaltyRegistry>,

    /// Immutable manifest binding.
    #[account(
        init,
        payer = payer,
        space = 8 + ManifestRegistry::INIT_SPACE,
        seeds = [MANIFEST_REGISTRY_SEED, asset_id.as_ref()],
        bump,
    )]
    pub manifest_registry: Account<'info, ManifestRegistry>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<InitRegistries>,
    asset_id: Pubkey,
    royalty_recipient: Pubkey,
    royalty_bps: u16,
    platform_fee_recipient: Pubkey,
    platform_fee_bps: u16,
    manifest_uri: String,
    manifest_sha256: [u8; 32],
    persona_id: String,
    spec_version: String,
) -> Result<()> {
    require!(royalty_bps <= 10_000, SaleError::PriceOverflow);
    require!(platform_fee_bps <= 10_000, SaleError::PriceOverflow);
    require!(
        royalty_bps as u32 + platform_fee_bps as u32 <= 10_000,
        SaleError::PriceOverflow
    );
    require!(manifest_uri.len() <= 256, SaleError::PriceOverflow);
    require!(persona_id.len() <= 48, SaleError::PriceOverflow);
    require!(spec_version.len() <= 8, SaleError::PriceOverflow);

    let now = Clock::get()?.slot;

    let r = &mut ctx.accounts.royalty_registry;
    r.asset_id = asset_id;
    r.royalty_recipient = royalty_recipient;
    r.royalty_bps = royalty_bps;
    r.platform_fee_recipient = platform_fee_recipient;
    r.platform_fee_bps = platform_fee_bps;
    r.created_at_slot = now;
    r.bump = ctx.bumps.royalty_registry;

    let m = &mut ctx.accounts.manifest_registry;
    m.asset_id = asset_id;
    m.manifest_uri = manifest_uri;
    m.manifest_sha256 = manifest_sha256;
    m.persona_id = persona_id;
    m.spec_version = spec_version;
    m.created_at_slot = now;
    m.bump = ctx.bumps.manifest_registry;

    Ok(())
}

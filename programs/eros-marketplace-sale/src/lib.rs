//! eros-marketplace-sale: atomic on-chain settlement for eros-nft v1 cards.
//!
//! See README.md and the spec at
//! eros-docs/docs/superpowers/specs/2026-05-09-eros-chat-marketplace-design.md.

use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod seeds;
pub mod state;

// Scaffold instruction kept for the existing litesvm integration test.
use instructions::*;

// Anchor 1.0's #[program] macro generates `pub use crate::__client_accounts_X::*` for each
// instruction's Context<X> type. Those `pub(crate) mod __client_accounts_X` modules are emitted
// adjacent to the `#[derive(Accounts)]` struct — i.e. inside the instruction submodules, not at
// the crate root. We re-export them here so the macro-generated code can resolve the paths.
pub(crate) use instructions::init_registries::__client_accounts_init_registries;
pub(crate) use instructions::initialize::__client_accounts_initialize;
pub(crate) use instructions::set_listing_quote::__client_accounts_set_listing_quote;

declare_id!("Ca8tTnDxUcXd1FKDaCc1x8m8faEU6NB3jfLhDNvrZK8a");

#[program]
pub mod eros_marketplace_sale {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::handler(ctx)
    }

    pub fn init_registries(
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
        instructions::init_registries::handler(
            ctx,
            asset_id,
            royalty_recipient,
            royalty_bps,
            platform_fee_recipient,
            platform_fee_bps,
            manifest_uri,
            manifest_sha256,
            persona_id,
            spec_version,
        )
    }

    pub fn set_listing_quote(
        ctx: Context<SetListingQuote>,
        asset_id: Pubkey,
        seller_wallet: Pubkey,
        listing_nonce: u64,
    ) -> Result<()> {
        instructions::set_listing_quote::handler(ctx, asset_id, seller_wallet, listing_nonce)
    }
}

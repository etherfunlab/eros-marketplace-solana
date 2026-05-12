//! eros-marketplace-solana: atomic on-chain settlement for eros-nft v1 cards.
//!
//! See README.md and the spec at
//! eros-docs/docs/superpowers/specs/2026-05-09-eros-chat-marketplace-design.md.

// Anchor 1.0's #[program] macro expansion trips
// `clippy::diverging_sub_expression` on the generated dispatch arms; this is a
// macro-side false positive we have no control over.
#![allow(clippy::diverging_sub_expression)]

use anchor_lang::prelude::*;

pub mod ed25519;
pub mod error;
pub mod instructions;
pub mod sale_order;
pub mod seeds;
pub mod state;

pub use sale_order::SaleOrder;

// Scaffold instruction kept for the existing litesvm integration test.
use instructions::*;

// Anchor 1.0's #[program] macro generates `pub use crate::__client_accounts_X::*` for each
// instruction's Context<X> type. Those `pub(crate) mod __client_accounts_X` modules are emitted
// adjacent to the `#[derive(Accounts)]` struct — i.e. inside the instruction submodules, not at
// the crate root. We re-export them here so the macro-generated code can resolve the paths.
pub(crate) use instructions::cancel_listing::__client_accounts_cancel_listing;
pub(crate) use instructions::execute_purchase::__client_accounts_execute_purchase;
pub(crate) use instructions::housekeeping_clear::__client_accounts_housekeeping_clear;
pub(crate) use instructions::init_registries::__client_accounts_init_registries;
pub(crate) use instructions::initialize::__client_accounts_initialize;
pub(crate) use instructions::set_listing_quote::__client_accounts_set_listing_quote;

declare_id!("Ca8tTnDxUcXd1FKDaCc1x8m8faEU6NB3jfLhDNvrZK8a");

#[program]
pub mod eros_marketplace_solana {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::handler(ctx)
    }

    #[allow(clippy::too_many_arguments)]
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

    pub fn cancel_listing(ctx: Context<CancelListing>) -> Result<()> {
        instructions::cancel_listing::handler(ctx)
    }

    pub fn housekeeping_clear(
        ctx: Context<HousekeepingClear>,
        asset_id: Pubkey,
        seller_wallet: Pubkey,
    ) -> Result<()> {
        instructions::housekeeping_clear::handler(ctx, asset_id, seller_wallet)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_purchase<'info>(
        ctx: Context<'info, ExecutePurchase<'info>>,
        sale_order: SaleOrder,
        ed25519_ix_index: u8,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> Result<()> {
        instructions::execute_purchase::handler(
            ctx,
            sale_order,
            ed25519_ix_index,
            root,
            data_hash,
            creator_hash,
            nonce,
            index,
        )
    }
}

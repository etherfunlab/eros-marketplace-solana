//! eros-marketplace-sale: atomic on-chain settlement for eros-nft v1 cards.
//!
//! See README.md and the spec at
//! eros-docs/docs/superpowers/specs/2026-05-09-eros-chat-marketplace-design.md.

use anchor_lang::prelude::*;

pub mod error;
pub mod seeds;
pub mod state;

// Scaffold instruction kept for the existing litesvm integration test.
mod instructions;
use instructions::*;

declare_id!("Ca8tTnDxUcXd1FKDaCc1x8m8faEU6NB3jfLhDNvrZK8a");

#[program]
pub mod eros_marketplace_sale {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::handler(ctx)
    }
}

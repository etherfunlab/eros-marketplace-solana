use anchor_lang::prelude::*;

use crate::error::SaleError;
use crate::seeds::{COLLECTION_REGISTRY_SEED, PROGRAM_CONFIG_SEED, SALE_AUTHORITY_SEED};
use crate::state::{CollectionRegistry, ProgramConfig};

/// Admin-gated registration of a Core collection. Captures the bump for the
/// `sale_authority` PDA derived as `[SALE_AUTHORITY_SEED, collection]` so
/// `execute_purchase` can sign as that PDA via O(1) bump lookup. The PDA is
/// the address that must be set as the Core collection's
/// `PermanentTransferDelegate` plugin (off-chain step performed by svc at
/// collection creation time).
#[derive(Accounts)]
#[instruction(collection: Pubkey)]
pub struct RegisterCollection<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub admin: Signer<'info>,

    #[account(
        seeds = [PROGRAM_CONFIG_SEED],
        bump = program_config.bump,
        has_one = admin @ SaleError::NotAdmin,
    )]
    pub program_config: Account<'info, ProgramConfig>,

    #[account(
        init,
        payer = payer,
        space = 8 + CollectionRegistry::INIT_SPACE,
        seeds = [COLLECTION_REGISTRY_SEED, collection.as_ref()],
        bump,
    )]
    pub collection_registry: Account<'info, CollectionRegistry>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<RegisterCollection>, collection: Pubkey) -> Result<()> {
    let (_sale_authority, sale_authority_bump) =
        Pubkey::find_program_address(&[SALE_AUTHORITY_SEED, collection.as_ref()], ctx.program_id);

    let r = &mut ctx.accounts.collection_registry;
    r.collection = collection;
    r.sale_authority_bump = sale_authority_bump;
    r.bump = ctx.bumps.collection_registry;
    r.registered_at = Clock::get()?.unix_timestamp;
    Ok(())
}

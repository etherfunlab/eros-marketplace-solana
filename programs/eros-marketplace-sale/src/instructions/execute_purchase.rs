use anchor_lang::prelude::*;
use solana_sdk_ids::sysvar::instructions::ID as INSTRUCTIONS_SYSVAR_ID;

use crate::ed25519::verify_ed25519_precompile;
use crate::error::SaleError;
use crate::sale_order::SaleOrder;
use crate::seeds::{LISTING_STATE_SEED, ROYALTY_REGISTRY_SEED};
use crate::state::{ListingState, RoyaltyRegistry};

#[derive(Accounts)]
#[instruction(sale_order: SaleOrder, ed25519_ix_index: u8)]
pub struct ExecutePurchase<'info> {
    /// Buyer; pays SOL for the purchase.
    #[account(mut)]
    pub buyer: Signer<'info>,

    /// CHECK: validated via SaleOrder.seller_wallet address constraint.
    /// Phase 6 will also verify via Bubblegum merkle proof cNFT owner check.
    #[account(mut, address = sale_order.seller_wallet)]
    pub seller: AccountInfo<'info>,

    /// CHECK: validated against royalty_registry.royalty_recipient in handler.
    #[account(mut)]
    pub royalty_recipient: AccountInfo<'info>,

    /// CHECK: validated against royalty_registry.platform_fee_recipient in handler.
    #[account(mut)]
    pub platform_fee_recipient: AccountInfo<'info>,

    /// Immutable royalty + platform fee parameters for this asset.
    #[account(
        seeds = [ROYALTY_REGISTRY_SEED, sale_order.asset_id.as_ref()],
        bump = royalty_registry.bump,
        constraint = royalty_registry.asset_id == sale_order.asset_id
            @ SaleError::RegistryAssetMismatch,
    )]
    pub royalty_registry: Account<'info, RoyaltyRegistry>,

    /// Mutable listing state — nonce is cleared on successful purchase.
    #[account(
        mut,
        seeds = [
            LISTING_STATE_SEED,
            sale_order.asset_id.as_ref(),
            sale_order.seller_wallet.as_ref(),
        ],
        bump = listing_state.bump,
    )]
    pub listing_state: Account<'info, ListingState>,

    /// Sysvar for Ed25519Program instruction introspection.
    /// CHECK: address is constrained to the well-known instructions sysvar ID.
    #[account(address = INSTRUCTIONS_SYSVAR_ID)]
    pub instructions_sysvar: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<ExecutePurchase>,
    sale_order: SaleOrder,
    ed25519_ix_index: u8,
) -> Result<()> {
    // 1. Verify that the Ed25519Program precompile instruction at ed25519_ix_index
    //    attests the seller's ed25519 signature over canonical(sale_order).
    let canonical = sale_order.canonical_bytes();
    verify_ed25519_precompile(
        &ctx.accounts.instructions_sysvar,
        ed25519_ix_index,
        &sale_order.seller_wallet,
        &canonical,
    )?;

    // 2. Verify expiry.
    let now = Clock::get()?.unix_timestamp;
    require!(sale_order.expires_at > now, SaleError::OrderExpired);

    // 3. Verify listing nonce matches the active nonce stored on-chain.
    let s = &mut ctx.accounts.listing_state;
    require!(
        s.active_nonce == Some(sale_order.listing_nonce),
        SaleError::ListingNonceMismatch
    );

    // 4. (Phase 6) On-chain cNFT owner check via Bubblegum merkle proof.
    //    For now, the `address = sale_order.seller_wallet` constraint on `seller`
    //    is the only enforcement.

    // 5. Verify royalty + platform fee recipients match the immutable registry.
    require_keys_eq!(
        ctx.accounts.royalty_recipient.key(),
        ctx.accounts.royalty_registry.royalty_recipient,
        SaleError::RegistryAssetMismatch
    );
    require_keys_eq!(
        ctx.accounts.platform_fee_recipient.key(),
        ctx.accounts.royalty_registry.platform_fee_recipient,
        SaleError::RegistryAssetMismatch
    );

    // 6. Compute SOL splits. All arithmetic is checked to prevent overflow.
    let r = &ctx.accounts.royalty_registry;
    let price = sale_order.price_lamports;
    let royalty = price
        .checked_mul(r.royalty_bps as u64)
        .ok_or(error!(SaleError::PriceOverflow))?
        / 10_000;
    let fee = price
        .checked_mul(r.platform_fee_bps as u64)
        .ok_or(error!(SaleError::PriceOverflow))?
        / 10_000;
    let proceeds = price
        .checked_sub(royalty)
        .ok_or(error!(SaleError::PriceOverflow))?
        .checked_sub(fee)
        .ok_or(error!(SaleError::PriceOverflow))?;

    // 7. Atomic SOL transfers via CPI to system_program (cleaner than direct
    //    lamport mutation — works for all SystemProgram-owned accounts).
    //    Anchor 1.0: CpiContext::new takes Pubkey (not AccountInfo) as first arg.
    use anchor_lang::system_program::{self, Transfer};
    let sys_id = anchor_lang::system_program::ID;

    if proceeds > 0 {
        system_program::transfer(
            CpiContext::new(
                sys_id,
                Transfer {
                    from: ctx.accounts.buyer.to_account_info(),
                    to: ctx.accounts.seller.to_account_info(),
                },
            ),
            proceeds,
        )?;
    }
    if royalty > 0 {
        system_program::transfer(
            CpiContext::new(
                sys_id,
                Transfer {
                    from: ctx.accounts.buyer.to_account_info(),
                    to: ctx.accounts.royalty_recipient.to_account_info(),
                },
            ),
            royalty,
        )?;
    }
    if fee > 0 {
        system_program::transfer(
            CpiContext::new(
                sys_id,
                Transfer {
                    from: ctx.accounts.buyer.to_account_info(),
                    to: ctx.accounts.platform_fee_recipient.to_account_info(),
                },
            ),
            fee,
        )?;
    }

    // 8. (Phase 6) Bubblegum cNFT transfer CPI signed by program PDA delegate.

    // 9. Clear the active listing nonce — prevents replay.
    ctx.accounts.listing_state.active_nonce = None;

    Ok(())
}

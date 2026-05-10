use anchor_lang::prelude::*;
use solana_sdk_ids::sysvar::instructions::ID as INSTRUCTIONS_SYSVAR_ID;

use crate::ed25519::verify_ed25519_precompile;
use crate::error::SaleError;
use crate::sale_order::SaleOrder;
use crate::seeds::{LISTING_STATE_SEED, ROYALTY_REGISTRY_SEED, SALE_AUTHORITY_SEED};
use crate::state::{ListingState, RoyaltyRegistry};

// ── mpl-bubblegum 3.0 CPI surface (kinobi-generated) ────────────────────────
//
// mpl-bubblegum 3.0 exposes a kinobi-generated CPI API.  Key types:
//
//   instructions::Transfer { tree_config, leaf_owner: (Pubkey, bool),
//       leaf_delegate: (Pubkey, bool), new_leaf_owner, merkle_tree,
//       log_wrapper, compression_program, system_program }
//   instructions::TransferInstructionArgs { root, data_hash, creator_hash,
//       nonce, index }
//   instructions::TransferCpi<'a,'b>  — holds &AccountInfo refs, exposes
//       invoke_signed_with_remaining_accounts(seeds, proof)
//   instructions::TransferCpiBuilder<'a,'b> — builder that calls TransferCpi
//   instructions::TransferBuilder — produces solana_program::Instruction from
//       Pubkeys (no lifetime issues)
//
// We use the low-level `Transfer::instruction_with_remaining_accounts` + a
// manual `invoke_signed` call to sidestep Rust's lifetime constraints that
// arise from `TransferCpi`'s borrowed `&'b AccountInfo<'a>` parameters.  This
// is semantically identical to using `TransferCpi::invoke_signed_with_remaining_accounts`.
//
// Discriminator (8-byte, Anchor-style): [163, 52, 200, 231, 140, 3, 69, 186]
//
// leaf_owner  = (seller, as_signer=false) — not signing; leaf delegate signs
// leaf_delegate = (sale_authority PDA, as_signer=true) — program PDA signs
// new_leaf_owner = buyer (read-only, no signature)
//
// Merkle proof path nodes are appended as remaining_accounts on the instruction
// (read-only, non-signer AccountMeta entries), in order from leaf to root.
//
// PDA seeds for sale_authority:
//   [SALE_AUTHORITY_SEED, asset_id.as_ref(), seller_wallet.as_ref(), &[bump]]
// ────────────────────────────────────────────────────────────────────────────

// Imports needed only when the Bubblegum CPI is compiled in.
#[cfg(not(feature = "test-without-bubblegum"))]
use mpl_bubblegum::instructions::{Transfer as BubblegumTransfer, TransferInstructionArgs};
#[cfg(not(feature = "test-without-bubblegum"))]
use anchor_lang::solana_program::{instruction::AccountMeta, program::invoke_signed};

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

    // ── Bubblegum transfer accounts (Phase 6) ──────────────────────────────
    //
    // The program PDA `sale_authority` acts as the leaf delegate that the seller
    // delegated to off-chain via a Bubblegum `delegate` ix. The PDA is unique per
    // (asset_id, seller_wallet) so different listings never share authority.

    /// Program PDA acting as Bubblegum leaf delegate.
    /// Seeds: [SALE_AUTHORITY_SEED, asset_id, seller_wallet]
    /// CHECK: This is a program-owned PDA. Its derivation is validated by Anchor's
    ///        `seeds` constraint. Bubblegum verifies the current delegate matches.
    #[account(
        seeds = [
            SALE_AUTHORITY_SEED,
            sale_order.asset_id.as_ref(),
            sale_order.seller_wallet.as_ref(),
        ],
        bump,
    )]
    pub sale_authority: AccountInfo<'info>,

    /// CHECK: Bubblegum tree config PDA for the merkle tree.
    ///        Validated by the Bubblegum program during CPI.
    #[account(mut)]
    pub tree_config: AccountInfo<'info>,

    /// CHECK: Concurrent merkle tree account for the cNFT.
    ///        Validated by the Bubblegum program during CPI.
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,

    /// CHECK: SPL Noop program (log_wrapper).
    ///        Bubblegum emits events via this program.
    pub log_wrapper: AccountInfo<'info>,

    /// CHECK: SPL Account Compression program.
    ///        Bubblegum delegates leaf mutation to this program.
    pub compression_program: AccountInfo<'info>,

    /// CHECK: mpl-bubblegum program.
    ///        Address is pinned to the canonical Bubblegum program ID.
    #[account(address = mpl_bubblegum::ID)]
    pub bubblegum_program: AccountInfo<'info>,
}

#[allow(clippy::too_many_arguments)]
pub fn handler<'info>(
    ctx: Context<'info, ExecutePurchase<'info>>,
    sale_order: SaleOrder,
    ed25519_ix_index: u8,
    root: [u8; 32],
    data_hash: [u8; 32],
    creator_hash: [u8; 32],
    nonce: u64,
    index: u32,
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

    // 4. Verify royalty + platform fee recipients match the immutable registry.
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

    // 5. Compute SOL splits. All arithmetic is checked to prevent overflow.
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

    // 6. Atomic SOL transfers via CPI to system_program (cleaner than direct
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

    // 7. Bubblegum cNFT transfer CPI signed by sale_authority PDA acting as delegate.
    //
    // This block is compiled out when the `test-without-bubblegum` feature is
    // active, allowing unit tests (solana-program-test) to exercise the SOL-split
    // and nonce-clearing logic without requiring real on-chain cNFT state.
    //
    // Implementation note: We use `BubblegumTransfer::instruction_with_remaining_accounts`
    // (which works with Pubkeys) rather than `TransferCpi` (which takes &AccountInfo refs)
    // to avoid Rust lifetime complexity from invariant generic parameters in Anchor's
    // `Context<T>` type.  The result is functionally identical.
    #[cfg(not(feature = "test-without-bubblegum"))]
    {
        // Derive the bump for sale_authority PDA so we can sign as the delegate.
        let sale_auth_bump = ctx.bumps.sale_authority;
        let asset_id_bytes = sale_order.asset_id.to_bytes();
        let seller_bytes = sale_order.seller_wallet.to_bytes();
        let bump_slice = [sale_auth_bump];
        let signer_seeds: &[&[u8]] = &[
            SALE_AUTHORITY_SEED,
            &asset_id_bytes,
            &seller_bytes,
            &bump_slice,
        ];

        // Build the Bubblegum transfer instruction using the Pubkey-based struct.
        //   leaf_owner = (seller, as_signer=false)  — delegate signs instead
        //   leaf_delegate = (sale_authority PDA, as_signer=true)
        //   new_leaf_owner = buyer
        let proof_metas: Vec<AccountMeta> = ctx
            .remaining_accounts
            .iter()
            .map(|a| AccountMeta::new_readonly(*a.key, false))
            .collect();

        let ix = BubblegumTransfer {
            tree_config: ctx.accounts.tree_config.key(),
            leaf_owner: (ctx.accounts.seller.key(), false),
            leaf_delegate: (ctx.accounts.sale_authority.key(), true),
            new_leaf_owner: ctx.accounts.buyer.key(),
            merkle_tree: ctx.accounts.merkle_tree.key(),
            log_wrapper: ctx.accounts.log_wrapper.key(),
            compression_program: ctx.accounts.compression_program.key(),
            system_program: ctx.accounts.system_program.key(),
        }
        .instruction_with_remaining_accounts(
            TransferInstructionArgs {
                root,
                data_hash,
                creator_hash,
                nonce,
                index,
            },
            &proof_metas,
        );

        // Collect account_infos in the same order as the instruction accounts.
        let mut account_infos = vec![
            ctx.accounts.bubblegum_program.to_account_info(), // program itself
            ctx.accounts.tree_config.to_account_info(),
            ctx.accounts.seller.to_account_info(),
            ctx.accounts.sale_authority.to_account_info(),
            ctx.accounts.buyer.to_account_info(),
            ctx.accounts.merkle_tree.to_account_info(),
            ctx.accounts.log_wrapper.to_account_info(),
            ctx.accounts.compression_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ];
        // Append proof node AccountInfos from remaining_accounts.
        for proof_acct in ctx.remaining_accounts.iter() {
            account_infos.push(proof_acct.to_account_info());
        }

        invoke_signed(&ix, &account_infos, &[signer_seeds])?;
    }

    // 8. Clear the active listing nonce — prevents replay.
    ctx.accounts.listing_state.active_nonce = None;

    Ok(())
}

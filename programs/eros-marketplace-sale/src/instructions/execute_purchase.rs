use anchor_lang::prelude::*;
use solana_sdk_ids::sysvar::instructions::ID as INSTRUCTIONS_SYSVAR_ID;

use crate::ed25519::verify_ed25519_precompile;
use crate::error::SaleError;
use crate::sale_order::SaleOrder;
use crate::seeds::{LISTING_STATE_SEED, ROYALTY_REGISTRY_SEED, SALE_AUTHORITY_SEED};
use crate::state::{ListingState, RoyaltyRegistry};

// ── mpl-bubblegum 3.0 CPI surface — TransferV2 ───────────────────────────────
//
// mpl-bubblegum 3.0 exposes a kinobi-generated CPI API.  We use the **V2**
// transfer instruction (`TransferV2`) which is the canonical instruction for
// V2-minted cNFTs (trees created with `create_tree_config_v2`).
//
// Key types:
//   instructions::TransferV2 { tree_config, payer, authority: Option<Pubkey>,
//       leaf_owner, leaf_delegate: Option<Pubkey>, new_leaf_owner, merkle_tree,
//       core_collection: Option<Pubkey>, log_wrapper, compression_program,
//       system_program }
//   instructions::TransferV2InstructionArgs { root, data_hash, creator_hash,
//       asset_data_hash: Option<[u8;32]>, flags: Option<u8>, nonce, index }
//
// Authority model (V2 vs V1):
//   V1 Transfer: leaf_delegate field must sign (as_signer=true). Our PDA acted
//     as the leaf delegate and signed. Straightforward delegate-based flow.
//
//   V2 TransferV2: `authority` (optional, defaults to `payer`) must be either
//     the leaf owner OR the collection's permanent transfer delegate. The
//     leaf_delegate field is now just informational/read-only. This means for a
//     program-mediated marketplace using V2:
//       - Set `authority = sale_authority PDA` (signs via PDA seeds).
//       - The collection MUST have `sale_authority` as its permanent transfer
//         delegate (set at collection creation time, not at listing time).
//       - `payer = buyer` (pays for the transaction).
//       - `leaf_owner = seller` (read-only).
//       - `leaf_delegate = sale_authority` (informational, read-only).
//
// Discriminator (8-byte, Anchor-style): [119, 40, 6, 235, 234, 221, 248, 49]
//
// Merkle proof path nodes are appended as remaining_accounts on the instruction
// (read-only, non-signer AccountMeta entries), in order from leaf to root.
//
// PDA seeds for sale_authority:
//   [SALE_AUTHORITY_SEED, asset_id.as_ref(), seller_wallet.as_ref(), &[bump]]
// ────────────────────────────────────────────────────────────────────────────

// Imports needed only when the Bubblegum CPI is compiled in.
#[cfg(not(feature = "test-without-bubblegum"))]
use mpl_bubblegum::instructions::{TransferV2 as BubblegumTransferV2, TransferV2InstructionArgs};
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

    // 7. Bubblegum cNFT TransferV2 CPI signed by sale_authority PDA.
    //
    // This block is compiled out when the `test-without-bubblegum` feature is
    // active, allowing unit tests (solana-program-test) to exercise the SOL-split
    // and nonce-clearing logic without requiring real on-chain cNFT state.
    //
    // Implementation note: We use `BubblegumTransferV2::instruction_with_remaining_accounts`
    // (which works with Pubkeys) rather than `TransferV2Cpi` (which takes &AccountInfo refs)
    // to avoid Rust lifetime complexity from invariant generic parameters in Anchor's
    // `Context<T>` type.  The result is functionally identical.
    //
    // V2 authority flow:
    //   payer     = buyer (writable, signer — pays the transaction)
    //   authority = sale_authority PDA (optional signer — must be leaf owner OR
    //               collection permanent transfer delegate; we use the collection
    //               permanent transfer delegate path: the marketplace collection
    //               must be created with sale_authority as its permanent transfer
    //               delegate)
    //   leaf_owner    = seller (read-only)
    //   leaf_delegate = sale_authority PDA (informational, read-only)
    //   new_leaf_owner = buyer
    #[cfg(not(feature = "test-without-bubblegum"))]
    {
        // Derive the bump for sale_authority PDA so we can sign as the authority.
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

        // Build the Bubblegum TransferV2 instruction using the Pubkey-based struct.
        let proof_metas: Vec<AccountMeta> = ctx
            .remaining_accounts
            .iter()
            .map(|a| AccountMeta::new_readonly(*a.key, false))
            .collect();

        let ix = BubblegumTransferV2 {
            tree_config: ctx.accounts.tree_config.key(),
            payer: ctx.accounts.buyer.key(),
            // authority = sale_authority PDA, which signs via PDA seeds.
            // In production, this PDA must be the collection's permanent transfer
            // delegate. The `None` fallback (→ payer) is wrong for marketplace use.
            authority: Some(ctx.accounts.sale_authority.key()),
            leaf_owner: ctx.accounts.seller.key(),
            // leaf_delegate is informational (read-only in V2).
            leaf_delegate: Some(ctx.accounts.sale_authority.key()),
            new_leaf_owner: ctx.accounts.buyer.key(),
            merkle_tree: ctx.accounts.merkle_tree.key(),
            // core_collection: None unless the cNFT has a Core collection.
            core_collection: None,
            log_wrapper: ctx.accounts.log_wrapper.key(),
            compression_program: ctx.accounts.compression_program.key(),
            system_program: ctx.accounts.system_program.key(),
        }
        .instruction_with_remaining_accounts(
            TransferV2InstructionArgs {
                root,
                data_hash,
                creator_hash,
                // asset_data_hash and flags are V2-only optional fields.
                // Pass None unless the cNFT uses Core asset-data extensions.
                asset_data_hash: None,
                flags: None,
                nonce,
                index,
            },
            &proof_metas,
        );

        // Collect account_infos in the same order as the TransferV2 instruction accounts:
        // [tree_config, payer, authority(opt), leaf_owner, leaf_delegate(opt),
        //  new_leaf_owner, merkle_tree, core_collection(opt), log_wrapper,
        //  compression_program, system_program, ...proof]
        //
        // When optional accounts are None, the kinobi-generated code substitutes
        // the Bubblegum program ID as a placeholder. We must replicate that here
        // for the AccountInfo slice.
        let mut account_infos = vec![
            ctx.accounts.bubblegum_program.to_account_info(), // program itself
            ctx.accounts.tree_config.to_account_info(),
            ctx.accounts.buyer.to_account_info(),      // payer (writable, signer)
            ctx.accounts.sale_authority.to_account_info(), // authority (signer via PDA)
            ctx.accounts.seller.to_account_info(),     // leaf_owner (read-only)
            ctx.accounts.sale_authority.to_account_info(), // leaf_delegate (read-only)
            ctx.accounts.buyer.to_account_info(),      // new_leaf_owner (read-only)
            ctx.accounts.merkle_tree.to_account_info(),
            // core_collection = None → Bubblegum program ID placeholder
            ctx.accounts.bubblegum_program.to_account_info(),
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

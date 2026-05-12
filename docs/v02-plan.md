# eros-marketplace-solana v0.2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign the marketplace program so its `sale_authority` PDA can act as a Core collection's `PermanentTransferDelegate`, unblocking Bubblegum V2 cNFT transfers and resolving Codex Critical #2.

**Architecture:** Replace the per-`(asset_id, seller_wallet)` `sale_authority` PDA with a per-`collection` PDA (`seeds = [b"sale_auth", collection_pubkey]`). Add a singleton-per-collection `CollectionRegistry` PDA (`seeds = [b"collection", collection_pubkey]`) registered by admin before any asset in that collection can be sold. `execute_purchase` accepts the collection as an account, verifies the registry exists, and signs the Bubblegum V2 `TransferV2` CPI as the registered authority. `SaleOrder` gains a `collection` field — seller signatures bind to (asset, collection, ...) so a malicious collection swap is rejected.

**Tech Stack:** Rust 1.89 + Anchor 1.0.2 (`anchor-lang`), mpl-bubblegum 3.0.0 (Rust crate; on-chain program `BGUMAp9...`), mpl-core 0.10 (Rust client used by svc; on-chain `CoREENxT...`), `solana-program-test` 3.x for unit tests. Mainnet for end-to-end validation (devnet's Bubblegum V2 is partially deployed — see probe 01).

**Spec:** `eros-reports/brainstorm/2026-05-12_marketplace_solana_v02_bubblegum_authority.md`
**Probe evidence:** `probes/01-collection-pda-delegate/README.md`

---

## File Map

| File | Action | Purpose |
|---|---|---|
| `programs/eros-marketplace-solana/Cargo.toml` | modify | Version `0.1.1-preaudit → 0.2.0-preaudit` |
| `programs/eros-marketplace-solana/src/seeds.rs` | modify | Add `COLLECTION_REGISTRY_SEED = b"collection"` |
| `programs/eros-marketplace-solana/src/state.rs` | modify | Add `CollectionRegistry` `#[account]` |
| `programs/eros-marketplace-solana/src/error.rs` | modify | Add `CollectionNotRegistered`, `CollectionMismatch` |
| `programs/eros-marketplace-solana/src/sale_order.rs` | modify | Add `collection: Pubkey` field; update `canonical_bytes` test |
| `programs/eros-marketplace-solana/src/instructions/mod.rs` | modify | Register `register_collection` module |
| `programs/eros-marketplace-solana/src/instructions/register_collection.rs` | **create** | New ix: admin-gated `CollectionRegistry` init |
| `programs/eros-marketplace-solana/src/instructions/execute_purchase.rs` | modify | New accounts (`collection_registry`, `core_collection`); change `sale_authority` seeds; pass `core_collection: Some(...)` to `TransferV2` CPI |
| `programs/eros-marketplace-solana/src/lib.rs` | modify | Add `register_collection` dispatcher arm + the `__client_accounts_register_collection` re-export |
| `program-tests/src/helpers.rs` | modify | Add `collection_registry_pda()`, `sale_authority_pda(collection)`, `register_collection_ix()`, `bootstrap_collection()` |
| `program-tests/src/lib.rs` | modify | Update existing tests to register a collection + add 2 new tests for `register_collection` |
| `CHANGELOG.md` | modify | v0.2.0-preaudit section |
| `probes/01-collection-pda-delegate/script/src/run.ts` | modify (in PR #1 follow-up) | Re-run on mainnet for the validation gate |

Tests touched: every existing test that calls `execute_purchase`, `init_registries`, or `set_listing_quote` must also bootstrap a `CollectionRegistry` for the test collection and embed `collection` in `SaleOrder`.

---

## Sequence Rationale

Tasks are ordered so each one builds on a green test suite. Strict TDD: red → green → commit. Each commit is small enough to revert. No task leaves the tree broken.

Strict order:
1. Scaffolding (seeds, errors) — no behavior change
2. `CollectionRegistry` state + `register_collection` ix — new surface, doesn't touch existing flow
3. `SaleOrder` field add — touches signature canonical bytes; tests for new shape
4. `execute_purchase` rewire — depends on 1–3
5. Test infrastructure catch-up + new test coverage
6. Version bump + CHANGELOG
7. Mainnet probe gate
8. Tag + GH prerelease

---

## Task 1: Bump pre-release suffix early

We mark the working version as `v0.2.0-preaudit` *before* the breaking changes land, so any intermediate failed CI run is clearly tagged "this is v0.2 in progress, not v0.1.1 master".

**Files:**
- Modify: `programs/eros-marketplace-solana/Cargo.toml:3`

- [ ] **Step 1: Change version**

```toml
[package]
name = "eros-marketplace-solana"
version = "0.2.0-preaudit"
description = "Atomic on-chain settlement for eros-nft v1 cards"
```

- [ ] **Step 2: Refresh Cargo.lock**

Run: `cargo update -p eros-marketplace-solana`
Expected: `Updating eros-marketplace-solana v0.1.1-preaudit -> v0.2.0-preaudit`

- [ ] **Step 3: Verify workspace still builds**

Run: `cargo check --workspace --all-targets`
Expected: clean exit code 0 with no errors (warnings ok)

- [ ] **Step 4: Commit**

```bash
git add programs/eros-marketplace-solana/Cargo.toml Cargo.lock
git commit -m "chore: bump version to 0.2.0-preaudit (v0.2 work begins)"
```

---

## Task 2: Add `COLLECTION_REGISTRY_SEED`

Pure constant — no behavior. Needed by Tasks 3 and 4.

**Files:**
- Modify: `programs/eros-marketplace-solana/src/seeds.rs`

- [ ] **Step 1: Add seed constant**

Append at end of file:

```rust
/// Derives the per-collection `CollectionRegistry` PDA. One per Core
/// collection; admin-registered before any asset in that collection can
/// be sold.
pub const COLLECTION_REGISTRY_SEED: &[u8] = b"collection";
```

Replace the doc comment on `SALE_AUTHORITY_SEED` to reflect the new derivation. The constant itself stays `b"sale_auth"` (program ID change would be far more disruptive than reusing the prefix).

```rust
/// Derives the program PDA that acts as Bubblegum V2 collection
/// `PermanentTransferDelegate` during `execute_purchase`.
///
/// Seeds: `[SALE_AUTHORITY_SEED, collection_pubkey]` — per-collection, not
/// per-`(asset_id, seller_wallet)`. The collection's `PermanentTransferDelegate`
/// plugin is set to this PDA at collection creation time.
pub const SALE_AUTHORITY_SEED: &[u8] = b"sale_auth";
```

- [ ] **Step 2: Verify clean build**

Run: `cargo build -p eros-marketplace-solana`
Expected: exit code 0

- [ ] **Step 3: Commit**

```bash
git add programs/eros-marketplace-solana/src/seeds.rs
git commit -m "feat: add COLLECTION_REGISTRY_SEED + redoc SALE_AUTHORITY_SEED"
```

---

## Task 3: Add `CollectionRegistry` state struct

**Files:**
- Modify: `programs/eros-marketplace-solana/src/state.rs`
- Test: `programs/eros-marketplace-solana/src/state.rs` (inline `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing test**

Append at end of `state.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_registry_init_space_is_stable() {
        // 32 (collection) + 1 (sale_authority_bump) + 1 (bump) + 8 (registered_at)
        // = 42 bytes payload; the 8-byte discriminator is added by Anchor.
        assert_eq!(CollectionRegistry::INIT_SPACE, 42);
    }
}
```

- [ ] **Step 2: Run to confirm fail**

Run: `cargo test -p eros-marketplace-solana --features test-without-bubblegum --lib state::tests::collection_registry_init_space_is_stable`
Expected: FAIL with `cannot find type CollectionRegistry`.

- [ ] **Step 3: Add the struct**

Append before the `#[cfg(test)]` block in `state.rs`:

```rust
/// Per-collection registry: binds a Core collection to its `sale_authority`
/// PDA (`seeds = [SALE_AUTHORITY_SEED, collection]`). Admin-initialized via
/// `register_collection` before any asset in the collection can be sold.
/// Storing `sale_authority_bump` here lets `execute_purchase` skip
/// `find_program_address` (O(1) PDA validation via `bump = ...`).
#[account]
#[derive(InitSpace)]
pub struct CollectionRegistry {
    pub collection: Pubkey,
    pub sale_authority_bump: u8,
    pub bump: u8,
    pub registered_at: i64, // unix timestamp seconds
}
```

- [ ] **Step 4: Run to confirm pass**

Run: `cargo test -p eros-marketplace-solana --features test-without-bubblegum --lib state::tests::collection_registry_init_space_is_stable`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add programs/eros-marketplace-solana/src/state.rs
git commit -m "feat: add CollectionRegistry account state"
```

---

## Task 4: Add error variants

**Files:**
- Modify: `programs/eros-marketplace-solana/src/error.rs`

- [ ] **Step 1: Add variants**

Insert before the closing `}` of `pub enum SaleError`:

```rust
    #[msg("CollectionRegistry not initialized for this collection — call register_collection first")]
    CollectionNotRegistered,

    #[msg("SaleOrder.collection does not match the provided collection account")]
    CollectionMismatch,

    #[msg("CollectionRegistry.collection does not match the provided collection account")]
    CollectionRegistryMismatch,
```

- [ ] **Step 2: Verify build**

Run: `cargo build -p eros-marketplace-solana`
Expected: exit code 0.

- [ ] **Step 3: Commit**

```bash
git add programs/eros-marketplace-solana/src/error.rs
git commit -m "feat: add CollectionNotRegistered, CollectionMismatch, CollectionRegistryMismatch errors"
```

---

## Task 5: `register_collection` ix — failing test first

**Files:**
- Create: `programs/eros-marketplace-solana/src/instructions/register_collection.rs`
- Modify: `programs/eros-marketplace-solana/src/instructions/mod.rs`
- Modify: `programs/eros-marketplace-solana/src/lib.rs`

- [ ] **Step 1: Write the integration test**

Add to `program-tests/src/lib.rs` at end:

```rust
#[tokio::test]
async fn register_collection_succeeds() {
    use crate::helpers::*;
    use solana_sdk::signer::Signer;
    let mut ctx = fresh_ctx().await;
    let payer = ctx.payer.insecure_clone();
    bootstrap_config(&mut ctx, &payer).await;

    let collection = solana_sdk::pubkey::Pubkey::new_unique();
    let ix = register_collection_ix(&payer.pubkey(), &payer.pubkey(), collection);
    send_tx(&mut ctx, &payer, &[ix]).await.expect("register_collection");

    // Verify the CollectionRegistry account exists at the expected PDA.
    let (registry_pda, _bump) = collection_registry_pda(&collection);
    let acct = ctx
        .banks_client
        .get_account(registry_pda)
        .await
        .unwrap()
        .expect("CollectionRegistry account missing");
    assert_eq!(acct.owner, eros_marketplace_solana::ID);
}

#[tokio::test]
async fn register_collection_rejects_wrong_admin() {
    use crate::helpers::*;
    use solana_sdk::signer::Signer;
    let mut ctx = fresh_ctx().await;
    let admin = ctx.payer.insecure_clone();
    bootstrap_config(&mut ctx, &admin).await;

    let imposter = solana_sdk::signature::Keypair::new();
    fund(&mut ctx, &imposter.pubkey(), 1_000_000_000).await;

    let collection = solana_sdk::pubkey::Pubkey::new_unique();
    // imposter as both payer + admin in the ix accounts, but ProgramConfig.admin
    // was captured as `admin` above — has_one check should reject.
    let ix = register_collection_ix(&imposter.pubkey(), &imposter.pubkey(), collection);
    let err = send_tx(&mut ctx, &imposter, &[ix]).await.expect_err("must fail");
    assert!(
        format!("{err:?}").contains("NotAdmin"),
        "expected NotAdmin, got: {err:?}"
    );
}
```

(Note: `register_collection_ix` and `collection_registry_pda` are added in Step 3 below. `fund` already exists in `helpers.rs` — verify with `grep "pub async fn fund" program-tests/src/helpers.rs`. If absent, add it per the snippet at the bottom of this task.)

- [ ] **Step 2: Confirm fail**

Run: `cargo build-sbf --manifest-path programs/eros-marketplace-solana/Cargo.toml --sbf-out-dir target/test-deploy --features test-without-bubblegum`
Then: `cargo test -p program-tests --lib register_collection`
Expected: BUILD FAIL on `register_collection_ix` / `collection_registry_pda` not found.

- [ ] **Step 3: Create the instruction module**

Create `programs/eros-marketplace-solana/src/instructions/register_collection.rs`:

```rust
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

    /// `init` — re-registering the same collection fails.
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
    let (_sale_authority, sale_authority_bump) = Pubkey::find_program_address(
        &[SALE_AUTHORITY_SEED, collection.as_ref()],
        ctx.program_id,
    );

    let r = &mut ctx.accounts.collection_registry;
    r.collection = collection;
    r.sale_authority_bump = sale_authority_bump;
    r.bump = ctx.bumps.collection_registry;
    r.registered_at = Clock::get()?.unix_timestamp;
    Ok(())
}
```

- [ ] **Step 4: Wire into mod.rs**

Modify `programs/eros-marketplace-solana/src/instructions/mod.rs` — add module declaration in alphabetical order alongside the others:

```rust
pub mod cancel_listing;
pub mod execute_purchase;
pub mod housekeeping_clear;
pub mod init_registries;
pub mod initialize;
pub mod register_collection;
pub mod set_listing_quote;

pub use cancel_listing::*;
pub use execute_purchase::*;
pub use housekeeping_clear::*;
pub use init_registries::*;
pub use initialize::*;
pub use register_collection::*;
pub use set_listing_quote::*;
```

(Check current state with `cat programs/eros-marketplace-solana/src/instructions/mod.rs` and follow its existing pattern — most likely it already has `pub mod x; pub use x::*;` per ix. Insert the two `register_collection` lines in the correct alphabetic slot.)

- [ ] **Step 5: Wire into `#[program]` dispatcher**

Modify `programs/eros-marketplace-solana/src/lib.rs`. Two changes:

(a) Add the re-export alongside the others (top of file, around the existing `pub(crate) use instructions::*::__client_accounts_*;` block):

```rust
pub(crate) use instructions::register_collection::__client_accounts_register_collection;
```

(b) Add the dispatcher arm inside `pub mod eros_marketplace_solana { ... }`, between `initialize` and `init_registries`:

```rust
    pub fn register_collection(
        ctx: Context<RegisterCollection>,
        collection: Pubkey,
    ) -> Result<()> {
        instructions::register_collection::handler(ctx, collection)
    }
```

- [ ] **Step 6: Add test helpers**

Modify `program-tests/src/helpers.rs` — append:

```rust
/// Derives the `CollectionRegistry` PDA for a given collection pubkey.
pub fn collection_registry_pda(collection: &Pubkey) -> (Pubkey, u8) {
    use eros_marketplace_solana::seeds::COLLECTION_REGISTRY_SEED;
    Pubkey::find_program_address(
        &[COLLECTION_REGISTRY_SEED, collection.as_ref()],
        &eros_marketplace_solana::ID,
    )
}

/// Derives the per-collection `sale_authority` PDA. v0.2: keyed by collection,
/// not by (asset, seller).
pub fn sale_authority_pda(collection: &Pubkey) -> (Pubkey, u8) {
    use eros_marketplace_solana::seeds::SALE_AUTHORITY_SEED;
    Pubkey::find_program_address(
        &[SALE_AUTHORITY_SEED, collection.as_ref()],
        &eros_marketplace_solana::ID,
    )
}

/// Builds the `register_collection` instruction.
pub fn register_collection_ix(
    payer: &Pubkey,
    admin: &Pubkey,
    collection: Pubkey,
) -> Instruction {
    use eros_marketplace_solana::accounts::RegisterCollection as Accounts_;
    use eros_marketplace_solana::instruction::RegisterCollection as Data_;

    let (program_config, _) = program_config_pda();
    let (collection_registry, _) = collection_registry_pda(&collection);
    let accounts = Accounts_ {
        payer: *payer,
        admin: *admin,
        program_config,
        collection_registry,
        system_program: anchor_lang::solana_program::system_program::ID,
    };
    Instruction {
        program_id: eros_marketplace_solana::ID,
        accounts: accounts.to_account_metas(None),
        data: Data_ { collection }.data(),
    }
}

/// Convenience: register a collection so subsequent `execute_purchase`
/// for assets in that collection works.
pub async fn bootstrap_collection(
    ctx: &mut ProgramTestContext,
    payer: &Keypair,
    collection: Pubkey,
) {
    let ix = register_collection_ix(&payer.pubkey(), &payer.pubkey(), collection);
    send_tx(ctx, payer, &[ix]).await.expect("bootstrap_collection");
}
```

If `fund` doesn't exist in `helpers.rs`, also add:

```rust
/// Fund a wallet with SOL from the test bank's airdrop.
pub async fn fund(ctx: &mut ProgramTestContext, recipient: &Pubkey, lamports: u64) {
    use solana_sdk::system_instruction;
    let payer = ctx.payer.insecure_clone();
    let ix = system_instruction::transfer(&payer.pubkey(), recipient, lamports);
    send_tx(ctx, &payer, &[ix]).await.expect("fund");
}
```

- [ ] **Step 7: Rebuild + test**

```bash
cargo build-sbf --manifest-path programs/eros-marketplace-solana/Cargo.toml --sbf-out-dir target/test-deploy --features test-without-bubblegum
cargo test -p program-tests --lib register_collection
```

Expected: 2/2 PASS for `register_collection_succeeds` and `register_collection_rejects_wrong_admin`.

- [ ] **Step 8: Commit**

```bash
git add programs/eros-marketplace-solana/src/instructions/register_collection.rs \
        programs/eros-marketplace-solana/src/instructions/mod.rs \
        programs/eros-marketplace-solana/src/lib.rs \
        program-tests/src/helpers.rs \
        program-tests/src/lib.rs
git commit -m "feat: register_collection ix + tests (CollectionRegistry init, admin-gated)"
```

---

## Task 6: Add `collection` field to `SaleOrder`

This is the off-chain signature breaking change. Once landed, **every seller-signed quote in the wild becomes invalid**. Since v0.1.1-preaudit was never on mainnet, the only impact is dev fixtures.

**Files:**
- Modify: `programs/eros-marketplace-solana/src/sale_order.rs`
- Test: `programs/eros-marketplace-solana/src/sale_order.rs` (inline tests)

- [ ] **Step 1: Update the failing tests first**

Replace the `#[cfg(test)]` block in `sale_order.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> SaleOrder {
        SaleOrder {
            asset_id: Pubkey::new_from_array([1u8; 32]),
            collection: Pubkey::new_from_array([9u8; 32]),
            seller_wallet: Pubkey::new_from_array([2u8; 32]),
            price_lamports: 1_000_000_000,
            listing_nonce: 42,
            expires_at: 1_700_000_000,
        }
    }

    #[test]
    fn canonical_bytes_is_deterministic() {
        let s = fixture();
        let a = s.canonical_bytes();
        let b = s.canonical_bytes();
        assert_eq!(a, b);
        // 32 + 32 + 32 + 8 + 8 + 8 = 120
        assert_eq!(a.len(), 120);
    }

    #[test]
    fn canonical_bytes_field_order_is_stable() {
        let s = fixture();
        let bytes = s.canonical_bytes();
        assert_eq!(&bytes[..32], &[1u8; 32]);     // asset_id
        assert_eq!(&bytes[32..64], &[9u8; 32]);   // collection
        assert_eq!(&bytes[64..96], &[2u8; 32]);   // seller_wallet
        assert_eq!(&bytes[96..104], &1_000_000_000u64.to_le_bytes());
        assert_eq!(&bytes[104..112], &42u64.to_le_bytes());
        assert_eq!(&bytes[112..120], &1_700_000_000i64.to_le_bytes());
    }
}
```

- [ ] **Step 2: Confirm fail**

Run: `cargo test -p eros-marketplace-solana --features test-without-bubblegum --lib sale_order::tests`
Expected: FAIL — `collection: Pubkey::...` not a recognized field of `SaleOrder`.

- [ ] **Step 3: Add the field**

Modify the struct definition in `sale_order.rs`:

```rust
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct SaleOrder {
    pub asset_id: Pubkey,
    /// Core collection the asset belongs to. Binds the seller signature to
    /// the (asset, collection) pair so a malicious collection-swap by the
    /// buyer is rejected (v0.2).
    pub collection: Pubkey,
    pub seller_wallet: Pubkey,
    pub price_lamports: u64,
    pub listing_nonce: u64,
    pub expires_at: i64,
}

impl SaleOrder {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        // 32 + 32 + 32 + 8 + 8 + 8 = 120 bytes
        let mut buf = Vec::with_capacity(32 + 32 + 32 + 8 + 8 + 8);
        self.serialize(&mut buf).expect("borsh serialize SaleOrder");
        buf
    }
}
```

- [ ] **Step 4: Confirm pass**

Run: `cargo test -p eros-marketplace-solana --features test-without-bubblegum --lib sale_order::tests`
Expected: 2/2 PASS.

- [ ] **Step 5: Commit**

```bash
git add programs/eros-marketplace-solana/src/sale_order.rs
git commit -m "feat!: SaleOrder gains collection field (canonical bytes 88->120, signature break)"
```

(`feat!` marks this as a breaking change.)

---

## Task 7: Update `execute_purchase` accounts struct

The structural change — sale_authority seeds + new accounts.

**Files:**
- Modify: `programs/eros-marketplace-solana/src/instructions/execute_purchase.rs`

- [ ] **Step 1: Replace the `#[derive(Accounts)] pub struct ExecutePurchase` block**

Find the `pub struct ExecutePurchase<'info>` block (around lines 54–143). Replace the `sale_authority`, `tree_config`, `merkle_tree`, etc. block with one that adds `collection_registry`, `core_collection`, and changes `sale_authority` seeds. Full replacement:

```rust
#[derive(Accounts)]
#[instruction(sale_order: SaleOrder, ed25519_ix_index: u8)]
pub struct ExecutePurchase<'info> {
    /// Buyer; pays SOL.
    #[account(mut)]
    pub buyer: Signer<'info>,

    /// CHECK: validated via SaleOrder.seller_wallet address constraint;
    /// Bubblegum verifies via merkle proof that the seller is the leaf owner.
    #[account(mut, address = sale_order.seller_wallet)]
    pub seller: UncheckedAccount<'info>,

    /// CHECK: validated against royalty_registry.royalty_recipient in handler.
    #[account(mut)]
    pub royalty_recipient: UncheckedAccount<'info>,

    /// CHECK: validated against royalty_registry.platform_fee_recipient in handler.
    #[account(mut)]
    pub platform_fee_recipient: UncheckedAccount<'info>,

    #[account(
        seeds = [ROYALTY_REGISTRY_SEED, sale_order.asset_id.as_ref()],
        bump = royalty_registry.bump,
        constraint = royalty_registry.asset_id == sale_order.asset_id
            @ SaleError::RegistryAssetMismatch,
    )]
    pub royalty_registry: Account<'info, RoyaltyRegistry>,

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

    /// Registry binding (collection ↔ sale_authority PDA). Existence proves
    /// admin registered this collection. seeds bind directly to
    /// `sale_order.collection`, so a buyer can't pass an unrelated registry.
    #[account(
        seeds = [COLLECTION_REGISTRY_SEED, sale_order.collection.as_ref()],
        bump = collection_registry.bump,
        constraint = collection_registry.collection == sale_order.collection
            @ SaleError::CollectionRegistryMismatch,
    )]
    pub collection_registry: Account<'info, CollectionRegistry>,

    /// CHECK: must match `sale_order.collection`. Passed through to Bubblegum
    /// V2 `TransferV2` CPI as the asset's Core collection; Bubblegum verifies
    /// the cNFT actually belongs to this collection via its V2 collection_hash.
    #[account(address = sale_order.collection @ SaleError::CollectionMismatch)]
    pub core_collection: UncheckedAccount<'info>,

    /// CHECK: well-known Ed25519Program instructions sysvar.
    #[account(address = INSTRUCTIONS_SYSVAR_ID)]
    pub instructions_sysvar: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    // ── Bubblegum V2 transfer accounts ──────────────────────────────────────
    /// Program PDA acting as the Core collection's `PermanentTransferDelegate`.
    /// Seeds: `[SALE_AUTHORITY_SEED, collection]` — keyed by collection only.
    /// CHECK: Anchor `seeds` constraint validates derivation; the bump comes
    /// from `collection_registry.sale_authority_bump` (cheaper than re-derive).
    #[account(
        seeds = [SALE_AUTHORITY_SEED, sale_order.collection.as_ref()],
        bump = collection_registry.sale_authority_bump,
    )]
    pub sale_authority: UncheckedAccount<'info>,

    /// CHECK: Bubblegum tree config PDA for the merkle tree. Validated by Bubblegum.
    #[account(mut)]
    pub tree_config: UncheckedAccount<'info>,

    /// CHECK: cMT for the cNFT. Validated by Bubblegum.
    #[account(mut)]
    pub merkle_tree: UncheckedAccount<'info>,

    /// CHECK: SPL Noop (log_wrapper).
    pub log_wrapper: UncheckedAccount<'info>,

    /// CHECK: mpl-account-compression (V2 compression program).
    pub compression_program: UncheckedAccount<'info>,

    /// CHECK: mpl-bubblegum program ID pinned.
    #[account(address = mpl_bubblegum::ID)]
    pub bubblegum_program: UncheckedAccount<'info>,
}
```

Add the new imports at the top of the file (modify the existing `use crate::seeds::...` and `use crate::state::...` lines):

```rust
use crate::seeds::{
    COLLECTION_REGISTRY_SEED, LISTING_STATE_SEED, ROYALTY_REGISTRY_SEED, SALE_AUTHORITY_SEED,
};
use crate::state::{CollectionRegistry, ListingState, Purchase, RoyaltyRegistry};
```

- [ ] **Step 2: Verify build (handler will still reference old account names, will fail next task)**

Run: `cargo build -p eros-marketplace-solana --features test-without-bubblegum`
Expected: build errors in the handler about old account field names. **That's OK — Task 8 fixes the handler.** Do NOT commit yet; commit after the handler change.

---

## Task 8: Rewrite `execute_purchase` handler for collection-scoped PDA

**Files:**
- Modify: `programs/eros-marketplace-solana/src/instructions/execute_purchase.rs`

- [ ] **Step 1: Update the handler signing seeds + CPI args**

Inside `pub fn handler<'info>(...)`, find the `#[cfg(not(feature = "test-without-bubblegum"))]` block. Replace its signer_seeds derivation and the `core_collection` field of the `BubblegumTransferV2` struct.

Old (delete):

```rust
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
```

New:

```rust
// v0.2: sale_authority PDA is keyed by collection, not by (asset, seller).
// The bump was captured into CollectionRegistry at register_collection time.
let sale_auth_bump = ctx.accounts.collection_registry.sale_authority_bump;
let collection_bytes = sale_order.collection.to_bytes();
let bump_slice = [sale_auth_bump];
let signer_seeds: &[&[u8]] = &[
    SALE_AUTHORITY_SEED,
    &collection_bytes,
    &bump_slice,
];
```

Then find the `BubblegumTransferV2 { ... }` literal. Replace `core_collection: None,` with `core_collection: Some(ctx.accounts.core_collection.key()),` AND in the `account_infos` vector replace the `core_collection = None → Bubblegum program ID placeholder` line:

Old:

```rust
core_collection: None,
```

```rust
// core_collection = None → Bubblegum program ID placeholder
ctx.accounts.bubblegum_program.to_account_info(),
```

New:

```rust
core_collection: Some(ctx.accounts.core_collection.key()),
```

```rust
ctx.accounts.core_collection.to_account_info(),
```

- [ ] **Step 2: Build the program**

Run: `cargo build -p eros-marketplace-solana --features test-without-bubblegum`
Expected: clean exit, no errors.

- [ ] **Step 3: Build the test .so**

Run: `cargo build-sbf --manifest-path programs/eros-marketplace-solana/Cargo.toml --sbf-out-dir target/test-deploy --features test-without-bubblegum`
Expected: clean.

- [ ] **Step 4: Confirm existing tests still pass** *(they will fail until Task 9 updates fixtures, then green)*

Run: `cargo test -p program-tests --lib`
Expected: failures in tests that call `execute_purchase` (SaleOrder missing `collection`, missing accounts). Tests that don't touch execute_purchase (register_collection ones, initialize) should still pass.

- [ ] **Step 5: Commit (after Task 9 fixes the tests)**

Defer commit to Task 9 step 5.

---

## Task 9: Update existing program-tests to match new ABI

Now make the test fixtures supply `collection` in SaleOrders and add the new accounts to `execute_purchase` ix calls.

**Files:**
- Modify: `program-tests/src/helpers.rs`
- Modify: `program-tests/src/lib.rs`

- [ ] **Step 1: Update the `execute_purchase_ix` helper**

Find `pub fn execute_purchase_ix(...)` in `helpers.rs`. The current signature builds the ix with only the v0.1.x accounts. Replace with:

```rust
#[allow(clippy::too_many_arguments)]
pub fn execute_purchase_ix(
    buyer: &Pubkey,
    sale_order: eros_marketplace_solana::sale_order::SaleOrder,
    ed25519_ix_index: u8,
    root: [u8; 32],
    data_hash: [u8; 32],
    creator_hash: [u8; 32],
    nonce: u64,
    index: u32,
    merkle_tree: Pubkey,
    tree_config: Pubkey,
    log_wrapper: Pubkey,
    compression_program: Pubkey,
) -> Instruction {
    use eros_marketplace_solana::accounts::ExecutePurchase as Accounts_;
    use eros_marketplace_solana::instruction::ExecutePurchase as Data_;

    let (royalty_registry, _) = royalty_registry_pda(&sale_order.asset_id);
    let (listing_state, _) = listing_state_pda(&sale_order.asset_id, &sale_order.seller_wallet);
    let (collection_registry, _) = collection_registry_pda(&sale_order.collection);
    let (sale_authority, _) = sale_authority_pda(&sale_order.collection);

    let accounts = Accounts_ {
        buyer: *buyer,
        seller: sale_order.seller_wallet,
        royalty_recipient: ROYALTY_RECIPIENT_FIXTURE,
        platform_fee_recipient: PLATFORM_FEE_RECIPIENT_FIXTURE,
        royalty_registry,
        listing_state,
        collection_registry,
        core_collection: sale_order.collection,
        instructions_sysvar: solana_sdk_ids::sysvar::instructions::ID,
        system_program: anchor_lang::solana_program::system_program::ID,
        sale_authority,
        tree_config,
        merkle_tree,
        log_wrapper,
        compression_program,
        bubblegum_program: mpl_bubblegum::ID,
    };
    Instruction {
        program_id: eros_marketplace_solana::ID,
        accounts: accounts.to_account_metas(None),
        data: Data_ {
            sale_order,
            ed25519_ix_index,
            root,
            data_hash,
            creator_hash,
            nonce,
            index,
        }
        .data(),
    }
}
```

(`ROYALTY_RECIPIENT_FIXTURE` and `PLATFORM_FEE_RECIPIENT_FIXTURE` are constants already in `helpers.rs`; verify with `grep _FIXTURE program-tests/src/helpers.rs` and reuse the existing values.)

- [ ] **Step 2: Update SaleOrder constructions in tests**

In `program-tests/src/lib.rs`, find every `SaleOrder { ... }` literal. Each needs a `collection: <pubkey>` field. Use a single shared fixture per test:

```rust
let collection = solana_sdk::pubkey::Pubkey::new_unique();
bootstrap_collection(&mut ctx, &payer, collection).await;

let so = eros_marketplace_solana::sale_order::SaleOrder {
    asset_id,
    collection,
    seller_wallet: seller.pubkey(),
    price_lamports: 1_000_000_000,
    listing_nonce: 1,
    expires_at: <now + 600>,
};
```

Apply this update to every `#[tokio::test]` that currently builds a SaleOrder. Search with: `grep -n "SaleOrder {" program-tests/src/lib.rs`.

- [ ] **Step 3: Add 2 new negative tests**

Append to `program-tests/src/lib.rs`:

```rust
#[tokio::test]
async fn execute_purchase_rejects_unregistered_collection() {
    use crate::helpers::*;
    use solana_sdk::signer::Signer;
    let mut ctx = fresh_ctx().await;
    let payer = ctx.payer.insecure_clone();
    bootstrap_config(&mut ctx, &payer).await;

    let seller = solana_sdk::signature::Keypair::new();
    let asset_id = Pubkey::new_unique();
    let collection = Pubkey::new_unique();          // intentionally NOT registered
    let registered_collection = Pubkey::new_unique();
    bootstrap_collection(&mut ctx, &payer, registered_collection).await; // a different one

    // Build a full happy-path execute_purchase except collection is unregistered.
    // We expect the Anchor `seeds` constraint on collection_registry to fail
    // because no account exists at the derived PDA — Anchor surfaces this as
    // `AccountNotInitialized` (3012) on the `collection_registry` account.
    let sale_order = eros_marketplace_solana::sale_order::SaleOrder {
        asset_id,
        collection,
        seller_wallet: seller.pubkey(),
        price_lamports: 1_000_000,
        listing_nonce: 1,
        expires_at: now_plus(&mut ctx, 600).await,
    };
    // Pre-init the asset's registries (admin = payer) and listing nonce — these
    // are orthogonal to the collection failure we're triggering.
    setup_purchase_fixtures(&mut ctx, &payer, &seller, &sale_order).await;

    let ix = execute_purchase_ix(
        &ctx.payer.pubkey(),
        sale_order,
        0,
        [0u8; 32], [0u8; 32], [0u8; 32], 0, 0,
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
    );
    let err = send_tx_with_ed25519(&mut ctx, &payer, &seller, &ix).await.expect_err("must fail");
    assert!(
        format!("{err:?}").contains("AccountNotInitialized") ||
            format!("{err:?}").contains("CollectionNotRegistered"),
        "got: {err:?}"
    );
}

#[tokio::test]
async fn execute_purchase_rejects_collection_account_mismatch() {
    use crate::helpers::*;
    use solana_sdk::signer::Signer;
    let mut ctx = fresh_ctx().await;
    let payer = ctx.payer.insecure_clone();
    bootstrap_config(&mut ctx, &payer).await;
    let signed_collection = Pubkey::new_unique();
    let swapped_collection = Pubkey::new_unique();
    bootstrap_collection(&mut ctx, &payer, signed_collection).await;
    bootstrap_collection(&mut ctx, &payer, swapped_collection).await;

    let seller = solana_sdk::signature::Keypair::new();
    let asset_id = Pubkey::new_unique();
    let mut sale_order = eros_marketplace_solana::sale_order::SaleOrder {
        asset_id,
        collection: signed_collection,
        seller_wallet: seller.pubkey(),
        price_lamports: 1_000_000,
        listing_nonce: 1,
        expires_at: now_plus(&mut ctx, 600).await,
    };
    setup_purchase_fixtures(&mut ctx, &payer, &seller, &sale_order).await;

    // Manually swap `core_collection` account in the AccountMeta vector to
    // a different (but also-registered) collection. The Anchor
    // `address = sale_order.collection` constraint should reject.
    let mut accs = build_execute_accounts(&mut ctx, &sale_order);
    accs.core_collection = swapped_collection;
    let ix = execute_purchase_ix_from_accounts(accs, sale_order.clone(), 0, [0u8; 32], [0u8; 32], [0u8; 32], 0, 0);

    let err = send_tx_with_ed25519(&mut ctx, &payer, &seller, &ix).await.expect_err("must fail");
    assert!(format!("{err:?}").contains("CollectionMismatch") || format!("{err:?}").contains("ConstraintAddress"));
}
```

(`now_plus`, `setup_purchase_fixtures`, `build_execute_accounts`, `execute_purchase_ix_from_accounts`, `send_tx_with_ed25519` — implement any that don't exist in `helpers.rs` as small forwarders. The first two should already exist for the v0.1.1 tests; verify with grep.)

- [ ] **Step 4: Rebuild + run all tests**

```bash
cargo build-sbf --manifest-path programs/eros-marketplace-solana/Cargo.toml --sbf-out-dir target/test-deploy --features test-without-bubblegum
cargo test -p program-tests --lib
```

Expected: all green. Should include 16 existing + 2 register_collection + 2 new negative = 20+ passing.

- [ ] **Step 5: Single commit for Tasks 7+8+9**

```bash
git add programs/eros-marketplace-solana/src/instructions/execute_purchase.rs \
        program-tests/src/helpers.rs \
        program-tests/src/lib.rs
git commit -m "feat!: execute_purchase uses collection-scoped sale_authority PDA + passes core_collection"
```

---

## Task 10: Update `CHANGELOG.md`

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Add v0.2.0-preaudit section**

Insert above the existing `## [0.1.1-preaudit] — 2026-05-12` section:

```markdown
## [0.2.0-preaudit] — TBD

⚠️ Still pre-audit. Do NOT deploy to mainnet without an external review of the
collection-permanent-delegate flow. The probe at
`probes/01-collection-pda-delegate/` validates the design on devnet (with
known gaps — see its README) and recommends a mainnet smoke before final
ship.

### Breaking changes (resolves Codex Critical #2)

- **`SaleOrder` canonical bytes change**: added `collection: Pubkey` field.
  Length 88 → 120 bytes. Every seller signature from v0.1.x is invalidated.
  v0.1.1-preaudit was never deployed to mainnet so no live signatures exist
  in the wild.
- **`execute_purchase` accounts surface**: added `collection_registry: Account<CollectionRegistry>` and `core_collection: UncheckedAccount`.
  `sale_authority` PDA seeds change from
  `[SALE_AUTH, asset_id, seller_wallet]` to `[SALE_AUTH, collection]`.
- **New instruction**: `register_collection(collection: Pubkey)` — admin-gated.
  Must be called once per Core collection before any asset in that collection
  can be sold via `execute_purchase`.
- **New on-chain account**: `CollectionRegistry { collection, sale_authority_bump, bump, registered_at }` at
  seeds `[b"collection", collection_pubkey]`.

### New errors

- `CollectionNotRegistered`
- `CollectionMismatch`
- `CollectionRegistryMismatch`

### Off-chain pipeline impact

The svc collection-onboarding flow now goes:

1. svc creates the Core collection via mpl-core `create_v2` with
   `PermanentTransferDelegate.authority = derive_sale_authority(collection)`.
2. svc calls `register_collection(collection)`.
3. svc proceeds with mint pipeline (Bubblegum V2 `create_tree_config_v2`,
   `mint_v2` with `core_collection`).

The `sale_authority` PDA is keyed by collection, so it's deterministic from
program_id + collection_pubkey without consulting any on-chain state.

### Carried from v0.1.1

- The pre-audit suffix stays until full audit (Soteria / Ottersec / Halborn).
- All Codex Critical/High items #1, #3, #4, #5 remain fixed; #2 is now resolved.
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs: changelog for v0.2.0-preaudit (collection-permanent-delegate)"
```

---

## Task 11: Push to a feature branch + open PR

Per session pattern (probe/01 PR is precedent): work on a feature branch and PR into main rather than pushing direct.

- [ ] **Step 1: Move work to a branch**

```bash
# Assumption: tasks 1-10 are committed locally on main; rewind main to origin/main
# and re-tip the new branch.
git checkout -b feat/v02-collection-authority
git checkout main
git reset --hard origin/main
git checkout feat/v02-collection-authority
```

- [ ] **Step 2: Push the branch**

```bash
git push -u origin feat/v02-collection-authority
```

- [ ] **Step 3: Open PR**

```bash
gh pr create --base main --head feat/v02-collection-authority --title "feat(v0.2): collection-permanent-delegate authority (Codex Critical #2)" --body "$(cat <<'EOF'
## Summary

Resolves the v0.1.1-preaudit carried Critical #2. Implements the design in
`eros-reports/brainstorm/2026-05-12_marketplace_solana_v02_bubblegum_authority.md`
(Option B — collection-scoped PDA).

## Architecture

- `sale_authority` PDA seeds: `[asset, seller] → [collection]`
- New `CollectionRegistry` PDA per collection; admin-gated registration
- `SaleOrder` gains `collection: Pubkey` — canonical bytes 88 → 120
- `execute_purchase` accepts `collection_registry` + `core_collection` and
  passes the latter into `TransferV2` CPI as `Some(...)`

## Test plan

- [x] All 16 v0.1.1 program-tests still green (with fixture updates)
- [x] `register_collection_succeeds`
- [x] `register_collection_rejects_wrong_admin`
- [x] `execute_purchase_rejects_unregistered_collection`
- [x] `execute_purchase_rejects_collection_account_mismatch`
- [ ] Mainnet probe validation gate — see Task 12 in `docs/v02-plan.md`

## Not in this PR

- svc-side collection-creation pipeline (different repo)
- IDL / TS client regen for downstream consumers (svc plan tracks)
EOF
)"
```

Expected: PR URL printed. Note the PR number for downstream tracking.

---

## Task 12: Mainnet probe — the validation gate

This is the single gate before tagging v0.2.0-preaudit as released. Cost: ~0.5 SOL. We rerun probe 01's Phase 1 against mainnet to confirm `MintV2` + `TransferV2` + `PermanentTransferDelegate` end-to-end work as the design expects.

**Why mainnet not devnet:** probe 01 confirmed devnet's Bubblegum V2 `MintV2` is broken; mainnet has 4+ successful MintV2 in the last 30 sigs (see probe 01 README "Cluster matrix").

**Why this is acceptable risk:** the probe uses a throwaway collection with a throwaway keypair. No production data. The collection becomes a permanent on-chain artifact but doesn't cost ongoing rent (rent-exempt once funded). The downside is the artifact is publicly visible and tied to our payer wallet — fine for a "probe" labeled collection.

**Files:**
- Modify: `probes/01-collection-pda-delegate/script/src/run.ts`
- Modify: `probes/01-collection-pda-delegate/README.md`
- Modify: `probes/01-collection-pda-delegate/.env`

- [ ] **Step 1: Point the script at mainnet**

Edit `probes/01-collection-pda-delegate/.env` (gitignored):

```
HELIUS_API_KEY=<your-mainnet-key>
SOLANA_RPC_URL=https://mainnet.helius-rpc.com/?api-key=<your-mainnet-key>
DAS_RPC_URL=https://mainnet.helius-rpc.com/?api-key=<your-mainnet-key>
```

In `run.ts`, also change the safety threshold (mainnet needs ~0.5 SOL, not 1.5):

```typescript
if (Number(balance.basisPoints) < 600_000_000) {
  throw new Error(
    `payer balance too low (need >=0.6 SOL on mainnet). got ${Number(balance.basisPoints) / 1e9}`,
  );
}
```

And change the collection's display name to make it obvious it's a probe:

```typescript
name: 'Probe01 MAINNET Collection (eros-marketplace v0.2 validation)',
```

- [ ] **Step 2: Fund the payer wallet**

```bash
solana config get  # confirm wallet path
solana balance     # confirm device wallet has >= 0.7 SOL mainnet
```

If insufficient, transfer from the user's main mainnet wallet (manual step). Do not airdrop on mainnet (not possible).

- [ ] **Step 3: Switch CLI to mainnet temporarily**

```bash
solana config set --url https://api.mainnet-beta.solana.com
solana balance
```

- [ ] **Step 4: Run the probe**

```bash
cd probes/01-collection-pda-delegate/script
export $(cat ../.env | xargs)
npx ts-node src/run.ts
```

Expected output (success): all phases A–H complete. Final SUMMARY shows:
- Q1 ✅
- Q2 ✅ (TransferV2 honors permanent delegate)
- Q3 ✅ (DAS reflects new owner within 15s)
- Q5 ✅
- Tx signatures linked to solscan with `?cluster=mainnet-beta` (will need a one-line tweak to the explorer URL — change `?cluster=devnet` to drop the query string or use mainnet).

- [ ] **Step 5: Capture DAS responses**

The probe already dumps `B-collection-account.json`, `E-das-initial.json`, `G-das-final.json`, `H-signatures-for-asset.json` into `helius-responses/`. After a successful run, **manually** copy these to a `helius-responses-mainnet/` subdir to preserve them alongside the devnet evidence:

```bash
cd probes/01-collection-pda-delegate
mkdir -p helius-responses-mainnet
cp helius-responses/*.json helius-responses-mainnet/
```

Add to git with `-f` (gitignored pattern matches them):

```bash
git add -f helius-responses-mainnet/
```

- [ ] **Step 6: Update probe README with mainnet results**

Replace the "Result (2026-05-12)" section's question matrix with the mainnet outcomes. Add a "Mainnet validation" subsection summarizing the SOL spent, collection address, DAS observations.

- [ ] **Step 7: Switch CLI back to devnet**

```bash
solana config set --url https://api.devnet.solana.com
```

- [ ] **Step 8: Commit + push to probe branch**

```bash
git checkout probe/01-collection-pda-delegate
git add probes/01-collection-pda-delegate/README.md probes/01-collection-pda-delegate/script/src/run.ts probes/01-collection-pda-delegate/helius-responses-mainnet/
git commit -m "probe(01): mainnet validation passes — v0.2 design end-to-end ✓"
git push origin probe/01-collection-pda-delegate
```

The PR for probe (#1) is updated automatically with the new commit.

---

## Task 13: Tag v0.2.0-preaudit

After the feat PR merges + probe mainnet validation is on the probe PR:

- [ ] **Step 1: Confirm CI green on main**

```bash
git checkout main
git pull origin main
gh run list --branch main --limit 1
```

Expected: latest run = success.

- [ ] **Step 2: Sign + push tag**

```bash
git tag -s v0.2.0-preaudit -m "v0.2.0-preaudit — collection-permanent-delegate (Codex Critical #2 resolved)"
git push origin v0.2.0-preaudit
```

- [ ] **Step 3: Create GH prerelease**

```bash
gh release create v0.2.0-preaudit \
    --verify-tag \
    --prerelease \
    --title "v0.2.0-preaudit" \
    --notes-file <(awk '/## \[0.2.0-preaudit\]/,/## \[0.1.1-preaudit\]/' CHANGELOG.md | head -n -1)
```

Expected: GH release URL printed.

---

## Tasks not in this plan

| Item | Owner | Tracked in |
|---|---|---|
| svc collection-creation pipeline (mpl-core `create_v2` + `PermanentTransferDelegate` plugin + `register_collection` call) | eros-marketplace-svc | `eros-docs/docs/superpowers/plans/2026-05-10-eros-marketplace-solana-plan.md` (rename + update for v0.2) |
| TS / JS client codegen against new IDL | downstream consumer | once feat PR merges, IDL is published as part of `target/idl/eros_marketplace_solana.json` |
| Audit engagement (Soteria / Ottersec / Halborn) | enriquephl | separate procurement track |
| Helius webhook → indexer event consumer (Phase 10 of svc) | eros-marketplace-svc | linked plan above |

---

## Self-Review

**Spec coverage**: brainstorm §3 Option B → Tasks 2, 3, 5, 7, 8 ✓. §4.1 CollectionRegistry → Task 3 ✓. §4.2 register_collection → Task 5 ✓. §4.3 execute_purchase changes → Tasks 7, 8 ✓. §4.5 IDL changes → CHANGELOG (Task 10) + the breaking `feat!` commits ✓. §5 svc impact → flagged as out-of-scope ✓. §6 deploy/migration → Task 13 ✓. §7 mainnet probe gate → Task 12 ✓.

**Placeholder scan**: TBD count in plan = 1 (the v0.2.0-preaudit release date in CHANGELOG, which is genuinely unknown until release day) — acceptable.

**Type consistency**: `CollectionRegistry.sale_authority_bump` (Task 3) consumed in `execute_purchase` via `ctx.accounts.collection_registry.sale_authority_bump` (Task 8) ✓. `register_collection_ix(payer, admin, collection)` (Task 5) consumed in tests (Tasks 5, 9) ✓. `collection_registry_pda(&collection)` (Task 5) consumed in `execute_purchase_ix` (Task 9) ✓.

**Test coverage gates**: every breaking change has a test. SaleOrder layout (Task 6, inline unit). Register collection happy + reject (Task 5). Execute purchase rejects unregistered + mismatched (Task 9).

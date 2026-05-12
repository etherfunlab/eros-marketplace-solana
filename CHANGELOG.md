# Changelog

## [0.2.0-preaudit] — TBD

⚠️ Still pre-audit. Do NOT deploy to mainnet without an external review of the
collection-permanent-delegate flow. The probe at
`probes/01-collection-pda-delegate/` validates the design on devnet (with
known gaps — see its README, particularly the Bubblegum V2 MintV2 status on
devnet). A mainnet smoke probe is the gate before final ship.

### Breaking changes (resolves Codex Critical #2)

- **`SaleOrder` canonical bytes change**: added `collection: Pubkey` as the
  second field (immediately after `asset_id`). Length 88 → 120 bytes.
  Every seller signature from v0.1.x is invalidated. v0.1.1-preaudit was
  never deployed to mainnet so no live signatures exist in the wild.
- **`execute_purchase` accounts surface**: added
  `collection_registry: Account<CollectionRegistry>` and
  `core_collection: UncheckedAccount`. The `sale_authority` PDA seeds change
  from `[SALE_AUTH, asset_id, seller_wallet]` to `[SALE_AUTH, collection]`,
  with the bump sourced from `CollectionRegistry.sale_authority_bump` for
  O(1) validation. The Bubblegum V2 `TransferV2` CPI now passes
  `core_collection: Some(...)` instead of `None`.
- **New instruction**: `register_collection(collection: Pubkey)` —
  admin-gated, `init` (not idempotent). Must be called once per Core
  collection before any asset in that collection can be sold via
  `execute_purchase`. Captures the `sale_authority` bump into
  `CollectionRegistry` at registration time.
- **New on-chain account**: `CollectionRegistry { collection,
  sale_authority_bump, bump, registered_at }` at seeds
  `[b"collection", collection_pubkey]`.

### New error variants

- `CollectionNotRegistered`
- `CollectionMismatch`
- `CollectionRegistryMismatch`

### Off-chain pipeline impact

The svc collection-onboarding flow now goes:

1. svc creates the Core collection via mpl-core `create_v2` with
   `PermanentTransferDelegate.authority = derive_sale_authority(collection)`.
2. svc calls `register_collection(collection)`.
3. svc proceeds with the Bubblegum V2 mint pipeline
   (`create_tree_config_v2`, `mint_v2` with `core_collection`).

The `sale_authority` PDA is keyed by collection only, so it's deterministic
from `program_id + collection_pubkey` without consulting any on-chain state.

### Tests

20 program-tests pass (16 carried over with collection threaded through,
2 new for `register_collection`, 2 new for `execute_purchase` negative cases
covering unregistered-collection and collection-account-mismatch). All under
the `test-without-bubblegum` feature flag; full Bubblegum V2 integration
exercised by `probes/01-collection-pda-delegate/` (devnet has a partial V2
deployment as of 2026-05-12 — the probe README documents this and the
mainnet validation gate).

### Carried from v0.1.1

- The pre-audit suffix stays until full audit (Soteria / Ottersec / Halborn).
- All Codex Critical/High items #1, #3, #4, #5 remain fixed; #2 is now
  resolved.

## [0.1.1-preaudit] — 2026-05-12

⚠️ Still pre-audit. Do NOT deploy to mainnet. The remaining Critical from
the v0.1.0 Codex review (#2 — Bubblegum V2 collection-permanent-delegate
authority model mismatch) is intentionally NOT patched here; it requires
architectural redesign and is deferred to v0.2.

### Security fixes (breaking)

- **Critical (#3 — Ed25519 signature bypass):** The precompile parser now
  requires all three descriptor `*_instruction_index` fields to equal
  `u16::MAX` and pins the canonical sig/pk/msg offset layout. The v0.1.0
  parser ignored those indices, letting a malicious buyer redirect the
  precompile's signature check at a sibling instruction's data while the
  handler read the seller's expected pubkey + canonical SaleOrder from
  this same instruction. New error variant: `Ed25519DescriptorMismatch`.
- **Critical (#1 — front-runnable immutable registries):** Added a
  singleton `ProgramConfig` PDA (seed `b"config"`) carrying an admin
  pubkey captured at `initialize` time. `init_registries` now requires
  an admin signer matching `ProgramConfig.admin`; v0.1.0 allowed any
  payer to permanently bind any asset_id to hostile royalty/manifest
  data.
- **High (#5 — set_listing_quote nonce grief):** Same admin gate as #1.
- `housekeeping_clear` migrated from the compile-time `ADMIN_WALLET`
  env-var pattern to the same `ProgramConfig` PDA so the program has one
  consistent auth surface.

### Features

- **High (#4 — Purchase event):** `execute_purchase` now emits an
  Anchor `Purchase` event after SOL splits + nonce clear, so the
  `eros-marketplace-svc` indexer (Phase 10) has something to parse from
  `Program data:` logs.

### Known issues carried to v0.2

- **Critical (#2 — Bubblegum V2 authority model):** `sale_authority` is
  derived per `(asset_id, seller)` but the README/plan claim it must be
  the collection-level permanent transfer delegate, which can only be a
  single program-wide PDA. `core_collection: None` is also wrong for
  Core-collection cNFTs. Requires a redesign around either a stable
  collection-level marketplace authority PDA or a seller/leaf-delegate
  flow Bubblegum V2 actually supports. Tracked for v0.2.

## [0.1.0] — 2026-05-10

Initial release.

- Five instructions: `init_registries`, `set_listing_quote`, `cancel_listing`,
  `housekeeping_clear`, `execute_purchase`.
- Three PDAs: `RoyaltyRegistry` (immutable royalty/fee binding),
  `ManifestRegistry` (immutable cNFT → Manifest binding), `ListingState`
  (mutable, monotonic nonce).
- Off-chain `SaleOrder` signed ed25519, verified on-chain via
  `Ed25519Program` precompile + instruction introspection.
- Atomic SOL splits + Bubblegum V2 TransferV2 cNFT transfer in `execute_purchase`.
- Full unit test coverage; smoke integration test on local validator.
- Anchor 1.0 + Solana 3.x (Agave) + mpl-bubblegum 3.0.
- Note: Full Bubblegum lifecycle integration test deferred to v0.2.
- Note: Devnet deployment deferred pending faucet rate-limit resolution.

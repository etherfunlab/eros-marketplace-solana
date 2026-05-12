# Changelog

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

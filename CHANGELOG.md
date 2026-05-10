# Changelog

## [Unreleased]

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

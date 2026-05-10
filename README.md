# eros-marketplace-sale

> Atomic on-chain settlement for [eros-nft](https://github.com/etherfunlab/eros-nft) v1 cards on Solana cNFT.

[![CI](https://github.com/etherfunlab/eros-marketplace-sale/actions/workflows/ci.yml/badge.svg)](https://github.com/etherfunlab/eros-marketplace-sale/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-Apache--2.0-green.svg)](LICENSE)

## What this is

Anchor program. Five instructions (`init_registries`, `set_listing_quote`,
`cancel_listing`, `housekeeping_clear`, `execute_purchase`). Three PDAs
(`RoyaltyRegistry`, `ManifestRegistry`, `ListingState`). Atomic SOL splits +
Bubblegum V2 cNFT transfer in one tx.

## Sale flow (high-level)

```
Off-chain (marketplace svc):
  - Seller signs a SaleOrder { asset_id, seller, price, nonce, expires_at }
    with their wallet's ed25519 key.
  - Listing goes live in DB + on-chain via set_listing_quote(nonce).
  - Seller delegates the leaf to the program PDA (Bubblegum delegate ix, same tx).

On-chain at purchase:
  - Buyer submits a tx with TWO instructions:
    1. Solana Ed25519Program: verifies seller's sig over canonical(SaleOrder).
    2. eros_marketplace_sale::execute_purchase: reads back the precompile,
       checks expiry/nonce/owner, computes royalty + platform fee + seller
       proceeds from the immutable RoyaltyRegistry, transfers SOL (3 outflows),
       and CPI-transfers the cNFT via Bubblegum (program PDA = leaf delegate).
```

## What this is NOT

- A marketplace service (catalog, listings UI, KMS) — see `eros-marketplace-svc`.
- A frontend — see `eros-engine-web` extensions.
- A persona standard — see [eros-nft](https://github.com/etherfunlab/eros-nft).

## Quick build + test

```bash
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
cargo install --git https://github.com/coral-xyz/anchor avm --locked
avm install 1.0.2 && avm use 1.0.2

anchor build
cargo test -p program-tests --lib --features eros-marketplace-sale/test-without-bubblegum
anchor test
```

## Known limitations

- **Bubblegum V2 authority setup:** The marketplace mint pipeline must set
  `sale_authority` (this program's PDA) as the collection's permanent transfer
  delegate at collection creation time. This is a Bubblegum V2 requirement;
  full integration is deferred to v0.2.
- **Devnet not yet deployed:** The v0.1.0 release was made with a rate-limited
  devnet faucet. Deployment details are in `docs/devnet-deploy-status.md`.
- **Full Bubblegum lifecycle integration test deferred to v0.2:** The current
  smoke test verifies `init_registries` on a local validator. A complete test
  covering mint → delegate → listing → purchase → owner verification is planned
  for v0.2.

## License

Apache-2.0. See `LICENSE`.

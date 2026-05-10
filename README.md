# eros-marketplace-sale

Anchor program for atomic settlement of `eros-nft` v1 cards on Solana cNFT.

This program handles ONLY the on-chain settlement layer: registering immutable
`RoyaltyRegistry` and `ManifestRegistry` PDAs at mint, tracking `ListingState`
with monotonic nonces, and executing buyer-submitted purchase transactions that
verify a seller-signed off-chain `SaleOrder`, atomically split SOL among
seller / royalty / platform recipients, and transfer the cNFT via Bubblegum V2
(program PDA acting as leaf delegate).

The off-chain marketplace service that orchestrates listings, KMS-encrypted
prompt delivery, and chat sessions is `eros-marketplace-svc` (separate repo).

## License

Apache-2.0. See `LICENSE`.

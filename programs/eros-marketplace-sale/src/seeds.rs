//! PDA seed-prefix constants. Tests import these.

pub const ROYALTY_REGISTRY_SEED: &[u8] = b"royalty";
pub const MANIFEST_REGISTRY_SEED: &[u8] = b"manifest";
pub const LISTING_STATE_SEED: &[u8] = b"listing";
/// Singleton program-wide configuration PDA holding the admin pubkey that
/// gates `init_registries`, `set_listing_quote`, and `housekeeping_clear`.
pub const PROGRAM_CONFIG_SEED: &[u8] = b"config";
/// Derives the program PDA that acts as Bubblegum leaf delegate during
/// `execute_purchase`. Seeded per (asset_id, seller_wallet) so each listing's
/// delegate is unique.
pub const SALE_AUTHORITY_SEED: &[u8] = b"sale_auth";

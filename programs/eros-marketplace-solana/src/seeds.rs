//! PDA seed-prefix constants. Tests import these.

pub const ROYALTY_REGISTRY_SEED: &[u8] = b"royalty";
pub const MANIFEST_REGISTRY_SEED: &[u8] = b"manifest";
pub const LISTING_STATE_SEED: &[u8] = b"listing";
/// Singleton program-wide configuration PDA holding the admin pubkey that
/// gates `init_registries`, `set_listing_quote`, and `housekeeping_clear`.
pub const PROGRAM_CONFIG_SEED: &[u8] = b"config";
/// Derives the program PDA that acts as Bubblegum V2 collection
/// `PermanentTransferDelegate` during `execute_purchase`.
///
/// Seeds: `[SALE_AUTHORITY_SEED, collection_pubkey]` — per-collection, not
/// per-`(asset_id, seller_wallet)`. The collection's `PermanentTransferDelegate`
/// plugin is set to this PDA at collection creation time.
pub const SALE_AUTHORITY_SEED: &[u8] = b"sale_auth";
/// Derives the per-collection `CollectionRegistry` PDA. One per Core
/// collection; admin-registered before any asset in that collection can
/// be sold.
pub const COLLECTION_REGISTRY_SEED: &[u8] = b"collection";

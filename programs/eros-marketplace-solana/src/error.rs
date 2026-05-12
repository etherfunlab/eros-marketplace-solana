use anchor_lang::prelude::*;

#[error_code]
pub enum SaleError {
    #[msg("listing_nonce must be strictly greater than last_seen_nonce")]
    NonceNotMonotonic,

    #[msg("no active listing for this (asset, seller)")]
    NoActiveListing,

    #[msg("SaleOrder.listing_nonce does not match active_nonce")]
    ListingNonceMismatch,

    #[msg("SaleOrder is past its expires_at")]
    OrderExpired,

    #[msg("seller signature over canonical SaleOrder is invalid or missing")]
    BadSellerSignature,

    #[msg("Ed25519Program precompile instruction not found at the expected position")]
    Ed25519PrecompileMissing,

    #[msg("Ed25519Program message bytes do not match canonical SaleOrder bytes")]
    Ed25519MessageMismatch,

    #[msg("Ed25519Program signing pubkey does not match SaleOrder.seller_wallet")]
    Ed25519PubkeyMismatch,

    #[msg("Ed25519Program descriptor uses non-canonical offsets or cross-instruction indices")]
    Ed25519DescriptorMismatch,

    #[msg("cNFT current owner does not match SaleOrder.seller_wallet")]
    OwnerMismatch,

    #[msg("provided RoyaltyRegistry asset_id does not match SaleOrder.asset_id")]
    RegistryAssetMismatch,

    #[msg("price computation overflowed")]
    PriceOverflow,

    #[msg("only the platform admin may call housekeeping_clear")]
    NotAdmin,

    #[msg("registry already initialized")]
    RegistryAlreadyInitialized,

    #[msg("CollectionRegistry not initialized for this collection — call register_collection first")]
    CollectionNotRegistered,

    #[msg("SaleOrder.collection does not match the provided collection account")]
    CollectionMismatch,

    #[msg("CollectionRegistry.collection does not match the provided collection account")]
    CollectionRegistryMismatch,
}

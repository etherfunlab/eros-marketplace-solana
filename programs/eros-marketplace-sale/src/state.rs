//! On-chain account layouts.

use anchor_lang::prelude::*;

/// Immutable per-asset royalty + platform fee binding. Initialized at mint by
/// the marketplace pipeline; no setter instruction exists.
#[account]
#[derive(InitSpace)]
pub struct RoyaltyRegistry {
    pub asset_id: Pubkey,
    pub royalty_recipient: Pubkey,
    pub royalty_bps: u16,             // basis points; e.g. 250 = 2.5%
    pub platform_fee_recipient: Pubkey,
    pub platform_fee_bps: u16,        // basis points; e.g. 500 = 5%
    pub created_at_slot: u64,
    pub bump: u8,
}

/// Immutable per-asset binding from cNFT to PersonaManifest. Initialized at
/// mint alongside RoyaltyRegistry. No setter exists.
#[account]
#[derive(InitSpace)]
pub struct ManifestRegistry {
    pub asset_id: Pubkey,
    #[max_len(256)]
    pub manifest_uri: String,         // e.g. "ar://abc..." (≤256 chars)
    pub manifest_sha256: [u8; 32],
    #[max_len(48)]
    pub persona_id: String,           // "ern:1.0:<26-char ULID>" = 34 chars; cap at 48
    #[max_len(8)]
    pub spec_version: String,         // e.g. "1.0"
    pub created_at_slot: u64,
    pub bump: u8,
}

/// Mutable per-(asset, seller) listing state. Tracks the active signed quote
/// nonce and a monotonic high-water mark to prevent nonce reuse.
#[account]
#[derive(InitSpace)]
pub struct ListingState {
    pub asset_id: Pubkey,
    pub seller_wallet: Pubkey,
    pub active_nonce: Option<u64>,    // None = no live listing
    pub last_seen_nonce: u64,         // monotonic; nonces MUST strictly increase
    pub bump: u8,
}

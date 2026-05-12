//! SaleOrder: the seller-signed off-chain quote.
//!
//! The signing message is the borsh-serialized canonical form. Borsh is
//! deterministic for the field types we use (u64, [u8;32], Pubkey).

use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct SaleOrder {
    pub asset_id: Pubkey,
    pub seller_wallet: Pubkey,
    pub price_lamports: u64,
    pub listing_nonce: u64,
    pub expires_at: i64,        // unix timestamp seconds
}

impl SaleOrder {
    /// Returns the canonical bytes that the seller signs.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        // AnchorSerialize is borsh; deterministic for these fields.
        let mut buf = Vec::with_capacity(32 + 32 + 8 + 8 + 8);
        self.serialize(&mut buf).expect("borsh serialize SaleOrder");
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> SaleOrder {
        SaleOrder {
            asset_id: Pubkey::new_from_array([1u8; 32]),
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
        // Spot-check length: 32 + 32 + 8 + 8 + 8 = 88
        assert_eq!(a.len(), 88);
    }

    #[test]
    fn canonical_bytes_field_order_is_stable() {
        // First 32 bytes must be asset_id
        let s = fixture();
        let bytes = s.canonical_bytes();
        assert_eq!(&bytes[..32], &[1u8; 32]);
        // Next 32 bytes seller_wallet
        assert_eq!(&bytes[32..64], &[2u8; 32]);
        // Next 8 bytes price_lamports (little-endian for borsh)
        assert_eq!(&bytes[64..72], &1_000_000_000u64.to_le_bytes());
        assert_eq!(&bytes[72..80], &42u64.to_le_bytes());
        assert_eq!(&bytes[80..88], &1_700_000_000i64.to_le_bytes());
    }
}

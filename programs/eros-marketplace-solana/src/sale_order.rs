//! SaleOrder: the seller-signed off-chain quote.
//!
//! The signing message is the borsh-serialized canonical form. Borsh is
//! deterministic for the field types we use (u64, [u8;32], Pubkey).

use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct SaleOrder {
    pub asset_id: Pubkey,
    /// Core collection the asset belongs to. Binds the seller signature to
    /// the (asset, collection) pair so a malicious collection-swap by the
    /// buyer is rejected (v0.2).
    pub collection: Pubkey,
    pub seller_wallet: Pubkey,
    pub price_lamports: u64,
    pub listing_nonce: u64,
    pub expires_at: i64, // unix timestamp seconds
}

impl SaleOrder {
    /// Returns the canonical bytes that the seller signs.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        // AnchorSerialize is borsh; deterministic for these fields.
        let mut buf = Vec::with_capacity(32 + 32 + 32 + 8 + 8 + 8);
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
            collection: Pubkey::new_from_array([9u8; 32]),
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
        // 32 + 32 + 32 + 8 + 8 + 8 = 120
        assert_eq!(a.len(), 120);
    }

    #[test]
    fn canonical_bytes_field_order_is_stable() {
        let s = fixture();
        let bytes = s.canonical_bytes();
        assert_eq!(&bytes[..32], &[1u8; 32]); // asset_id
        assert_eq!(&bytes[32..64], &[9u8; 32]); // collection
        assert_eq!(&bytes[64..96], &[2u8; 32]); // seller_wallet
        assert_eq!(&bytes[96..104], &1_000_000_000u64.to_le_bytes());
        assert_eq!(&bytes[104..112], &42u64.to_le_bytes());
        assert_eq!(&bytes[112..120], &1_700_000_000i64.to_le_bytes());
    }
}

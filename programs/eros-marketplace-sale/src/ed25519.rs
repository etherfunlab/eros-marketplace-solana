//! Ed25519Program precompile introspection.
//!
//! The buyer's tx must contain BOTH:
//!   1. An `Ed25519Program` instruction with seller's pubkey + canonical
//!      SaleOrder bytes + signature.
//!   2. Our `execute_purchase` instruction.
//!
//! Solana validates the Ed25519Program instruction itself before our ix runs.
//! We then look up that instruction (by index) and verify it carries the
//! expected pubkey + message.
//!
//! Layout of an Ed25519Program instruction's data (little-endian):
//!   - u8: number of signatures (we expect 1)
//!   - u8: padding
//!   - For each sig:
//!     - u16 signature_offset
//!     - u16 signature_instruction_index (0xFFFF = same instruction)
//!     - u16 public_key_offset
//!     - u16 public_key_instruction_index
//!     - u16 message_data_offset
//!     - u16 message_data_size
//!     - u16 message_instruction_index
//!   - signature bytes (64)
//!   - public key bytes (32)
//!   - message bytes (variable)

use anchor_lang::prelude::*;
// In Anchor 1.0 / Solana 3.x the anchor_lang::solana_program re-export does not
// expose load_instruction_at_checked or ed25519_program; use the disaggregated
// Solana crates directly (both are transitive deps of anchor-lang 1.0).
use solana_instructions_sysvar::load_instruction_at_checked;
use solana_sdk_ids::ed25519_program;

use crate::error::SaleError;

/// Verifies that the instruction at `ed25519_ix_index` is a valid
/// Ed25519Program instruction with the expected `expected_pubkey` and
/// `expected_message`. Returns Ok if all match; Err otherwise.
pub fn verify_ed25519_precompile(
    instructions_sysvar: &AccountInfo<'_>,
    ed25519_ix_index: u8,
    expected_pubkey: &Pubkey,
    expected_message: &[u8],
) -> Result<()> {
    let ix = load_instruction_at_checked(ed25519_ix_index as usize, instructions_sysvar)
        .map_err(|_| error!(SaleError::Ed25519PrecompileMissing))?;

    require_keys_eq!(
        ix.program_id,
        ed25519_program::ID,
        SaleError::Ed25519PrecompileMissing
    );

    let data = &ix.data;
    require!(data.len() >= 16, SaleError::Ed25519PrecompileMissing);
    require_eq!(data[0], 1u8, SaleError::Ed25519PrecompileMissing); // exactly 1 sig

    // Parse the offsets (little-endian u16)
    let pk_off = u16::from_le_bytes([data[6], data[7]]) as usize;
    let msg_off = u16::from_le_bytes([data[10], data[11]]) as usize;
    let msg_size = u16::from_le_bytes([data[12], data[13]]) as usize;

    require!(
        pk_off + 32 <= data.len() && msg_off + msg_size <= data.len(),
        SaleError::Ed25519PrecompileMissing
    );

    let actual_pubkey = &data[pk_off..pk_off + 32];
    require!(
        actual_pubkey == expected_pubkey.as_ref(),
        SaleError::Ed25519PubkeyMismatch
    );

    let actual_msg = &data[msg_off..msg_off + msg_size];
    require!(
        actual_msg == expected_message,
        SaleError::Ed25519MessageMismatch
    );

    Ok(())
}

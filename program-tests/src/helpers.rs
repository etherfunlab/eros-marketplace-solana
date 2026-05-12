//! Shared test utilities for eros_marketplace_sale program tests.

use anchor_lang::{InstructionData, ToAccountMetas};
use eros_marketplace_sale::seeds::{
    MANIFEST_REGISTRY_SEED, PROGRAM_CONFIG_SEED, ROYALTY_REGISTRY_SEED,
};
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};

/// Spins up a fresh in-process bank with our program loaded.
///
/// Sets `SBF_OUT_DIR` to `target/test-deploy` — a dedicated directory that
/// contains the program `.so` compiled WITH the `test-without-bubblegum`
/// feature flag so the Bubblegum CPI block is compiled out.
///
/// **Build requirement**: before running `cargo test -p program-tests`, run:
///   ```
///   cargo build-sbf \
///       --manifest-path programs/eros-marketplace-sale/Cargo.toml \
///       --sbf-out-dir target/test-deploy \
///       --features test-without-bubblegum
///   ```
///
/// Why not use the production `.so` from `target/deploy`?
/// `anchor build` compiles without `test-without-bubblegum`, so that `.so`
/// has the full Bubblegum CPI block. When the test bank tries to invoke the
/// Bubblegum program ID (which isn't loaded), it fails with `UnsupportedProgramId`.
/// A dedicated test `.so` avoids this without requiring the inline-processor
/// approach (which has incompatibilities with Anchor 1.0's CPI syscall stubs
/// in `solana-program-test 3.x`).
pub async fn fresh_ctx() -> ProgramTestContext {
    std::env::set_var(
        "SBF_OUT_DIR",
        concat!(env!("CARGO_MANIFEST_DIR"), "/../target/test-deploy"),
    );
    let pt = ProgramTest::new("eros_marketplace_sale", eros_marketplace_sale::ID, None);
    pt.start_with_context().await
}

/// Derives the singleton `ProgramConfig` PDA.
pub fn program_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PROGRAM_CONFIG_SEED], &eros_marketplace_sale::ID)
}

/// Builds the `initialize` ix. `admin` is captured into the ProgramConfig PDA
/// and gates all subsequent privileged instructions.
pub fn initialize_ix(payer: &Pubkey, admin: &Pubkey) -> Instruction {
    use eros_marketplace_sale::accounts::Initialize as Accounts_;
    use eros_marketplace_sale::instruction::Initialize as Data_;

    let (program_config, _) = program_config_pda();
    let accounts = Accounts_ {
        payer: *payer,
        admin: *admin,
        program_config,
        system_program: anchor_lang::solana_program::system_program::ID,
    };
    Instruction {
        program_id: eros_marketplace_sale::ID,
        accounts: accounts.to_account_metas(None),
        data: Data_ {}.data(),
    }
}

/// Convenience: send `initialize` so the ProgramConfig PDA exists with
/// `payer` as both rent-payer and admin. Returns nothing on success.
pub async fn bootstrap_config(ctx: &mut ProgramTestContext, payer: &Keypair) {
    let ix = initialize_ix(&payer.pubkey(), &payer.pubkey());
    send_tx(ctx, payer, &[ix]).await.expect("bootstrap_config");
}

/// Derives the `RoyaltyRegistry` PDA for a given `asset_id`.
pub fn royalty_registry_pda(asset_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[ROYALTY_REGISTRY_SEED, asset_id.as_ref()],
        &eros_marketplace_sale::ID,
    )
}

/// Derives the `ManifestRegistry` PDA for a given `asset_id`.
pub fn manifest_registry_pda(asset_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[MANIFEST_REGISTRY_SEED, asset_id.as_ref()],
        &eros_marketplace_sale::ID,
    )
}

/// Builds the `init_registries` instruction using Anchor's generated client types.
#[allow(clippy::too_many_arguments)]
pub fn init_registries_ix(
    payer: &Pubkey,
    admin: &Pubkey,
    asset_id: Pubkey,
    royalty_recipient: Pubkey,
    royalty_bps: u16,
    platform_fee_recipient: Pubkey,
    platform_fee_bps: u16,
    manifest_uri: String,
    manifest_sha256: [u8; 32],
    persona_id: String,
    spec_version: String,
) -> Instruction {
    use eros_marketplace_sale::accounts::InitRegistries as InitRegistriesAccounts;
    use eros_marketplace_sale::instruction::InitRegistries as InitRegistriesData;

    let (royalty_pda, _) = royalty_registry_pda(&asset_id);
    let (manifest_pda, _) = manifest_registry_pda(&asset_id);
    let (program_config, _) = program_config_pda();

    let accounts = InitRegistriesAccounts {
        payer: *payer,
        admin: *admin,
        program_config,
        royalty_registry: royalty_pda,
        manifest_registry: manifest_pda,
        system_program: anchor_lang::solana_program::system_program::ID,
    };
    let data = InitRegistriesData {
        asset_id,
        royalty_recipient,
        royalty_bps,
        platform_fee_recipient,
        platform_fee_bps,
        manifest_uri,
        manifest_sha256,
        persona_id,
        spec_version,
    };

    Instruction {
        program_id: eros_marketplace_sale::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

use eros_marketplace_sale::seeds::LISTING_STATE_SEED;

/// Derives the `ListingState` PDA for a given `(asset_id, seller)` pair.
pub fn listing_state_pda(asset_id: &Pubkey, seller: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[LISTING_STATE_SEED, asset_id.as_ref(), seller.as_ref()],
        &eros_marketplace_sale::ID,
    )
}

/// Builds the `set_listing_quote` instruction using Anchor's generated client types.
pub fn set_listing_quote_ix(
    payer: &Pubkey,
    admin: &Pubkey,
    asset_id: Pubkey,
    seller_wallet: Pubkey,
    listing_nonce: u64,
) -> Instruction {
    use eros_marketplace_sale::accounts::SetListingQuote as Accounts_;
    use eros_marketplace_sale::instruction::SetListingQuote as Data_;

    let (listing_pda, _) = listing_state_pda(&asset_id, &seller_wallet);
    let (program_config, _) = program_config_pda();

    let accounts = Accounts_ {
        payer: *payer,
        admin: *admin,
        program_config,
        listing_state: listing_pda,
        system_program: anchor_lang::solana_program::system_program::ID,
    };
    let data = Data_ {
        asset_id,
        seller_wallet,
        listing_nonce,
    };

    Instruction {
        program_id: eros_marketplace_sale::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Builds the `cancel_listing` instruction using Anchor's generated client types.
pub fn cancel_listing_ix(seller: &Pubkey, listing_pda: Pubkey) -> Instruction {
    use eros_marketplace_sale::accounts::CancelListing as Accounts_;
    use eros_marketplace_sale::instruction::CancelListing as Data_;

    let accounts = Accounts_ {
        seller: *seller,
        listing_state: listing_pda,
    };
    let data = Data_ {};

    Instruction {
        program_id: eros_marketplace_sale::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Builds the `housekeeping_clear` instruction using Anchor's generated client types.
pub fn housekeeping_clear_ix(
    admin: &Pubkey,
    asset_id: Pubkey,
    seller_wallet: Pubkey,
) -> Instruction {
    use eros_marketplace_sale::accounts::HousekeepingClear as Accounts_;
    use eros_marketplace_sale::instruction::HousekeepingClear as Data_;

    let (listing_pda, _) = listing_state_pda(&asset_id, &seller_wallet);
    let (program_config, _) = program_config_pda();

    let accounts = Accounts_ {
        admin: *admin,
        program_config,
        listing_state: listing_pda,
    };
    let data = Data_ {
        asset_id,
        seller_wallet,
    };

    Instruction {
        program_id: eros_marketplace_sale::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    }
}

/// Builds and submits a transaction with `payer` as the fee-payer and sole signer.
pub async fn send_tx(
    ctx: &mut ProgramTestContext,
    payer: &Keypair,
    ixs: &[Instruction],
) -> Result<(), solana_program_test::BanksClientError> {
    let recent = ctx.last_blockhash;
    let tx = Transaction::new_signed_with_payer(ixs, Some(&payer.pubkey()), &[payer], recent);
    ctx.banks_client.process_transaction(tx).await
}

// ── Phase 5: execute_purchase helpers ───────────────────────────────────────

use eros_marketplace_sale::SaleOrder;

/// Manually constructs an Ed25519Program instruction whose data layout matches
/// what `verify_ed25519_precompile` parses:
///   [0]    u8: count (1)
///   [1]    u8: padding (0)
///   [2..4] u16 le: sig_offset (= 16)
///   [4..6] u16 le: sig_ix_index (= 0xFFFF, same-ix)
///   [6..8] u16 le: pk_offset (= 80)
///   [8..10] u16 le: pk_ix_index (= 0xFFFF)
///   [10..12] u16 le: msg_offset (= 112)
///   [12..14] u16 le: msg_size
///   [14..16] u16 le: msg_ix_index (= 0xFFFF)
///   [16..80]  sig bytes (64)
///   [80..112] pubkey bytes (32)
///   [112..]   message bytes
pub fn ed25519_precompile_ix(
    pubkey: &[u8; 32],
    signature: &[u8; 64],
    message: &[u8],
) -> Instruction {
    // offsets into the data blob (everything lives in the same instruction)
    let sig_off: u16 = 16;
    let pk_off: u16 = sig_off + 64;   // 80
    let msg_off: u16 = pk_off + 32;   // 112
    let msg_size: u16 = message.len() as u16;
    let same_ix: u16 = u16::MAX;

    let mut data = Vec::with_capacity(16 + 64 + 32 + message.len());
    // header
    data.push(1u8);  // count
    data.push(0u8);  // padding
    // signature descriptor
    data.extend_from_slice(&sig_off.to_le_bytes());
    data.extend_from_slice(&same_ix.to_le_bytes());
    // pubkey descriptor
    data.extend_from_slice(&pk_off.to_le_bytes());
    data.extend_from_slice(&same_ix.to_le_bytes());
    // message descriptor
    data.extend_from_slice(&msg_off.to_le_bytes());
    data.extend_from_slice(&msg_size.to_le_bytes());
    data.extend_from_slice(&same_ix.to_le_bytes());
    // payload
    data.extend_from_slice(signature);
    data.extend_from_slice(pubkey);
    data.extend_from_slice(message);

    Instruction {
        program_id: solana_sdk_ids::ed25519_program::ID,
        accounts: vec![],
        data,
    }
}

/// Variant of `ed25519_precompile_ix` that lets the caller poison one or more
/// of the three descriptor `*_instruction_index` fields (sig / pk / msg). Used
/// by negative tests for the cross-instruction signature-bypass attack.
pub fn ed25519_precompile_ix_with_indices(
    pubkey: &[u8; 32],
    signature: &[u8; 64],
    message: &[u8],
    sig_ix_index: u16,
    pk_ix_index: u16,
    msg_ix_index: u16,
) -> Instruction {
    let sig_off: u16 = 16;
    let pk_off: u16 = sig_off + 64;   // 80
    let msg_off: u16 = pk_off + 32;   // 112
    let msg_size: u16 = message.len() as u16;

    let mut data = Vec::with_capacity(16 + 64 + 32 + message.len());
    data.push(1u8);
    data.push(0u8);
    data.extend_from_slice(&sig_off.to_le_bytes());
    data.extend_from_slice(&sig_ix_index.to_le_bytes());
    data.extend_from_slice(&pk_off.to_le_bytes());
    data.extend_from_slice(&pk_ix_index.to_le_bytes());
    data.extend_from_slice(&msg_off.to_le_bytes());
    data.extend_from_slice(&msg_size.to_le_bytes());
    data.extend_from_slice(&msg_ix_index.to_le_bytes());
    data.extend_from_slice(signature);
    data.extend_from_slice(pubkey);
    data.extend_from_slice(message);

    Instruction {
        program_id: solana_sdk_ids::ed25519_program::ID,
        accounts: vec![],
        data,
    }
}

// ── Phase 6: Bubblegum placeholder accounts for unit tests ──────────────────
//
// When the `test-without-bubblegum` feature is active (as it is for all unit
// tests in this crate), the Bubblegum CPI block is compiled out but the
// accounts struct still needs to be populated. `BubblegumPlaceholders` provides
// dummy values so existing tests can pass `BubblegumPlaceholders::default()`.

use eros_marketplace_sale::seeds::SALE_AUTHORITY_SEED;

/// Placeholder Bubblegum accounts for unit tests.
///
/// All pubkeys are unique sentinels — the on-chain program accepts them because
/// the CPI block is gated behind `#[cfg(not(feature = "test-without-bubblegum"))]`.
#[derive(Debug)]
pub struct BubblegumPlaceholders {
    pub sale_authority: Pubkey,
    pub tree_config: Pubkey,
    pub merkle_tree: Pubkey,
    pub log_wrapper: Pubkey,
    pub compression_program: Pubkey,
    pub bubblegum_program: Pubkey,
    pub root: [u8; 32],
    pub data_hash: [u8; 32],
    pub creator_hash: [u8; 32],
    pub nonce: u64,
    pub index: u32,
    /// Merkle proof path nodes (empty for unit tests).
    pub proof: Vec<Pubkey>,
}

impl BubblegumPlaceholders {
    /// Derive the real sale_authority PDA for a given (asset_id, seller) pair.
    /// Use this when you want the PDA to match what the program derives on-chain.
    pub fn with_pda(asset_id: &Pubkey, seller: &Pubkey) -> Self {
        let (sale_authority, _) = Pubkey::find_program_address(
            &[SALE_AUTHORITY_SEED, asset_id.as_ref(), seller.as_ref()],
            &eros_marketplace_sale::ID,
        );
        Self {
            sale_authority,
            ..Self::default()
        }
    }
}

impl Default for BubblegumPlaceholders {
    fn default() -> Self {
        Self {
            // Any pubkey for PDA placeholder — no PDA check when CPI is gated.
            sale_authority: Pubkey::new_unique(),
            tree_config: Pubkey::new_unique(),
            merkle_tree: Pubkey::new_unique(),
            log_wrapper: Pubkey::new_unique(),
            compression_program: Pubkey::new_unique(),
            // Use the real Bubblegum program ID; the address constraint on the
            // account always validates it even in test-without-bubblegum mode.
            bubblegum_program: mpl_bubblegum::ID,
            root: [0u8; 32],
            data_hash: [0u8; 32],
            creator_hash: [0u8; 32],
            nonce: 0,
            index: 0,
            proof: vec![],
        }
    }
}

/// Derives the `sale_authority` PDA for a given `(asset_id, seller)` pair.
pub fn sale_authority_pda(asset_id: &Pubkey, seller: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[SALE_AUTHORITY_SEED, asset_id.as_ref(), seller.as_ref()],
        &eros_marketplace_sale::ID,
    )
}

/// Builds an `execute_purchase` instruction.
///
/// `bb` carries the Bubblegum-related accounts and proof args. Under the
/// `test-without-bubblegum` feature flag, the Bubblegum CPI is skipped
/// on-chain, so dummy values from `BubblegumPlaceholders::default()` work.
pub fn execute_purchase_ix(
    buyer: &Pubkey,
    seller: &Pubkey,
    royalty_recipient: &Pubkey,
    platform_fee_recipient: &Pubkey,
    sale_order: SaleOrder,
    ed25519_ix_index: u8,
    bb: BubblegumPlaceholders,
) -> Instruction {
    use eros_marketplace_sale::accounts::ExecutePurchase as Accounts_;
    use eros_marketplace_sale::instruction::ExecutePurchase as Data_;

    let (royalty_pda, _) = royalty_registry_pda(&sale_order.asset_id);
    let (listing_pda, _) = listing_state_pda(&sale_order.asset_id, &sale_order.seller_wallet);

    // Derive the real sale_authority PDA so it satisfies the seeds constraint.
    let (sale_authority, _) = sale_authority_pda(&sale_order.asset_id, seller);

    let accounts = Accounts_ {
        buyer: *buyer,
        seller: *seller,
        royalty_recipient: *royalty_recipient,
        platform_fee_recipient: *platform_fee_recipient,
        royalty_registry: royalty_pda,
        listing_state: listing_pda,
        instructions_sysvar: solana_sdk_ids::sysvar::instructions::ID,
        system_program: anchor_lang::solana_program::system_program::ID,
        sale_authority,
        tree_config: bb.tree_config,
        merkle_tree: bb.merkle_tree,
        log_wrapper: bb.log_wrapper,
        compression_program: bb.compression_program,
        bubblegum_program: bb.bubblegum_program,
    };
    let data = Data_ {
        sale_order,
        ed25519_ix_index,
        root: bb.root,
        data_hash: bb.data_hash,
        creator_hash: bb.creator_hash,
        nonce: bb.nonce,
        index: bb.index,
    };

    let mut ix = Instruction {
        program_id: eros_marketplace_sale::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };

    // Append proof nodes as additional read-only non-signer accounts.
    for proof_node in &bb.proof {
        ix.accounts.push(solana_sdk::instruction::AccountMeta::new_readonly(
            *proof_node,
            false,
        ));
    }

    ix
}

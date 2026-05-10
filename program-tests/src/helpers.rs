//! Shared test utilities for eros_marketplace_sale program tests.

use anchor_lang::{InstructionData, ToAccountMetas};
use eros_marketplace_sale::seeds::{MANIFEST_REGISTRY_SEED, ROYALTY_REGISTRY_SEED};
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
/// Sets `SBF_OUT_DIR` so `solana-program-test` can locate the compiled `.so`
/// artifact produced by `anchor build` (same workaround used in Phase 1).
pub async fn fresh_ctx() -> ProgramTestContext {
    std::env::set_var(
        "SBF_OUT_DIR",
        concat!(env!("CARGO_MANIFEST_DIR"), "/../target/deploy"),
    );
    let pt = ProgramTest::new("eros_marketplace_sale", eros_marketplace_sale::ID, None);
    pt.start_with_context().await
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

    let accounts = InitRegistriesAccounts {
        payer: *payer,
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
    asset_id: Pubkey,
    seller_wallet: Pubkey,
    listing_nonce: u64,
) -> Instruction {
    use eros_marketplace_sale::accounts::SetListingQuote as Accounts_;
    use eros_marketplace_sale::instruction::SetListingQuote as Data_;

    let (listing_pda, _) = listing_state_pda(&asset_id, &seller_wallet);

    let accounts = Accounts_ {
        payer: *payer,
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

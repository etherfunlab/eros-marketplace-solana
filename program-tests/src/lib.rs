//! Rust-side integration tests for eros_marketplace_sale.
//! Each top-level test function is `#[tokio::test]` and uses solana-program-test
//! to spin a lightweight in-process bank with our program loaded.

#[cfg(test)]
mod helpers;

#[cfg(test)]
mod tests {
    use super::helpers::*;
    use eros_marketplace_sale::state::{ManifestRegistry, RoyaltyRegistry};
    use solana_sdk::pubkey::Pubkey;
    use solana_sdk::signer::Signer;

    fn sample_init_args() -> (
        Pubkey, // asset_id
        Pubkey, // royalty_recipient
        u16,    // royalty_bps
        Pubkey, // platform_fee_recipient
        u16,    // platform_fee_bps
        String, // manifest_uri
        [u8; 32], // manifest_sha256
        String, // persona_id
        String, // spec_version
    ) {
        let asset_id = Pubkey::new_unique();
        let royalty_recipient = Pubkey::new_unique();
        let platform_fee_recipient = Pubkey::new_unique();
        (
            asset_id,
            royalty_recipient,
            250,
            platform_fee_recipient,
            500,
            "ar://abc".to_string(),
            [0u8; 32],
            "ern:1.0:01HXY0000000000000000000Y1".to_string(),
            "1.0".to_string(),
        )
    }

    /// Proves the harness wires up — no assertions, just startup.
    #[tokio::test]
    async fn harness_compiles_and_runs() {
        // Use None processor and set SBF_OUT_DIR so solana-program-test can
        // locate the compiled .so artifact produced by `anchor build`.
        std::env::set_var(
            "SBF_OUT_DIR",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../target/deploy"),
        );
        let _ctx = fresh_ctx().await;
    }

    /// Happy path: init_registries writes both PDAs correctly.
    #[tokio::test]
    async fn init_registries_happy_path() {
        let mut ctx = fresh_ctx().await;
        let payer = ctx.payer.insecure_clone();
        let (asset_id, rr, rb, pf, pb, mu, ms, pi, sv) = sample_init_args();

        let ix = init_registries_ix(
            &payer.pubkey(),
            asset_id,
            rr,
            rb,
            pf,
            pb,
            mu.clone(),
            ms,
            pi.clone(),
            sv.clone(),
        );
        send_tx(&mut ctx, &payer, &[ix]).await.expect("init ok");

        // Read back royalty registry and assert all fields
        let (rpda, _) = royalty_registry_pda(&asset_id);
        let acct = ctx
            .banks_client
            .get_account(rpda)
            .await
            .unwrap()
            .expect("royalty_registry account must exist");
        let r: RoyaltyRegistry =
            anchor_lang::AccountDeserialize::try_deserialize(&mut acct.data.as_slice()).unwrap();
        assert_eq!(r.asset_id, asset_id);
        assert_eq!(r.royalty_recipient, rr);
        assert_eq!(r.royalty_bps, 250);
        assert_eq!(r.platform_fee_recipient, pf);
        assert_eq!(r.platform_fee_bps, 500);

        // Read back manifest registry and assert all fields
        let (mpda, _) = manifest_registry_pda(&asset_id);
        let acct = ctx
            .banks_client
            .get_account(mpda)
            .await
            .unwrap()
            .expect("manifest_registry account must exist");
        let m: ManifestRegistry =
            anchor_lang::AccountDeserialize::try_deserialize(&mut acct.data.as_slice()).unwrap();
        assert_eq!(m.asset_id, asset_id);
        assert_eq!(m.manifest_uri, mu);
        assert_eq!(m.manifest_sha256, ms);
        assert_eq!(m.persona_id, pi);
        assert_eq!(m.spec_version, sv);
    }

    /// Double-init rejection: calling init_registries twice for the same asset_id must fail.
    #[tokio::test]
    async fn init_registries_double_init_fails() {
        let mut ctx = fresh_ctx().await;
        let payer = ctx.payer.insecure_clone();
        let (asset_id, rr, rb, pf, pb, mu, ms, pi, sv) = sample_init_args();

        // First init: must succeed
        let ix = init_registries_ix(
            &payer.pubkey(),
            asset_id,
            rr,
            rb,
            pf,
            pb,
            mu.clone(),
            ms,
            pi.clone(),
            sv.clone(),
        );
        send_tx(&mut ctx, &payer, &[ix.clone()])
            .await
            .expect("first init ok");

        // Refresh the blockhash so the second tx is distinct
        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

        // Second init with the same accounts: must fail (Anchor `init` rejects existing accounts)
        let result = send_tx(&mut ctx, &payer, &[ix]).await;
        assert!(
            result.is_err(),
            "second init must fail because the PDA already exists"
        );
    }
}

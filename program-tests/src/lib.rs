//! Rust-side integration tests for eros_marketplace_solana.
//! Each top-level test function is `#[tokio::test]` and uses solana-program-test
//! to spin a lightweight in-process bank with our program loaded.

#[cfg(test)]
mod helpers;

#[cfg(test)]
mod tests {
    use super::helpers::*;
    use ed25519_dalek::{Signer as DalekSigner, SigningKey};
    use eros_marketplace_solana::state::{ManifestRegistry, RoyaltyRegistry};
    use eros_marketplace_solana::SaleOrder;
    use solana_sdk::pubkey::Pubkey;
    use solana_sdk::signature::Keypair;
    use solana_sdk::signer::Signer;
    use solana_sdk::transaction::Transaction;

    fn sample_init_args() -> (
        Pubkey,   // asset_id
        Pubkey,   // royalty_recipient
        u16,      // royalty_bps
        Pubkey,   // platform_fee_recipient
        u16,      // platform_fee_bps
        String,   // manifest_uri
        [u8; 32], // manifest_sha256
        String,   // persona_id
        String,   // spec_version
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
        bootstrap_config(&mut ctx, &payer).await;
        let (asset_id, rr, rb, pf, pb, mu, ms, pi, sv) = sample_init_args();

        let ix = init_registries_ix(
            &payer.pubkey(),
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

    use eros_marketplace_solana::state::ListingState;

    async fn read_listing(
        ctx: &mut solana_program_test::ProgramTestContext,
        pda: Pubkey,
    ) -> ListingState {
        let acct = ctx
            .banks_client
            .get_account(pda)
            .await
            .unwrap()
            .expect("listing exists");
        anchor_lang::AccountDeserialize::try_deserialize(&mut acct.data.as_slice()).unwrap()
    }

    #[tokio::test]
    async fn set_listing_quote_first_call_initializes() {
        let mut ctx = fresh_ctx().await;
        let payer = ctx.payer.insecure_clone();
        bootstrap_config(&mut ctx, &payer).await;
        let asset_id = Pubkey::new_unique();
        let seller = Pubkey::new_unique();

        let ix = set_listing_quote_ix(&payer.pubkey(), &payer.pubkey(), asset_id, seller, 1);
        send_tx(&mut ctx, &payer, &[ix])
            .await
            .expect("first listing ok");

        let (pda, _) = listing_state_pda(&asset_id, &seller);
        let s = read_listing(&mut ctx, pda).await;
        assert_eq!(s.asset_id, asset_id);
        assert_eq!(s.seller_wallet, seller);
        assert_eq!(s.active_nonce, Some(1));
        assert_eq!(s.last_seen_nonce, 1);
    }

    #[tokio::test]
    async fn set_listing_quote_relisting_advances_nonce() {
        let mut ctx = fresh_ctx().await;
        let payer = ctx.payer.insecure_clone();
        bootstrap_config(&mut ctx, &payer).await;
        let asset_id = Pubkey::new_unique();
        let seller = Pubkey::new_unique();

        send_tx(
            &mut ctx,
            &payer,
            &[set_listing_quote_ix(
                &payer.pubkey(),
                &payer.pubkey(),
                asset_id,
                seller,
                1,
            )],
        )
        .await
        .unwrap();

        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        send_tx(
            &mut ctx,
            &payer,
            &[set_listing_quote_ix(
                &payer.pubkey(),
                &payer.pubkey(),
                asset_id,
                seller,
                5,
            )],
        )
        .await
        .unwrap();

        let (pda, _) = listing_state_pda(&asset_id, &seller);
        let s = read_listing(&mut ctx, pda).await;
        assert_eq!(s.active_nonce, Some(5));
        assert_eq!(s.last_seen_nonce, 5);
    }

    #[tokio::test]
    async fn set_listing_quote_rejects_non_monotonic_nonce() {
        let mut ctx = fresh_ctx().await;
        let payer = ctx.payer.insecure_clone();
        bootstrap_config(&mut ctx, &payer).await;
        let asset_id = Pubkey::new_unique();
        let seller = Pubkey::new_unique();

        send_tx(
            &mut ctx,
            &payer,
            &[set_listing_quote_ix(
                &payer.pubkey(),
                &payer.pubkey(),
                asset_id,
                seller,
                5,
            )],
        )
        .await
        .unwrap();

        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

        // Equal nonce: must fail (must be strictly greater)
        let result = send_tx(
            &mut ctx,
            &payer,
            &[set_listing_quote_ix(
                &payer.pubkey(),
                &payer.pubkey(),
                asset_id,
                seller,
                5,
            )],
        )
        .await;
        assert!(result.is_err(), "equal nonce must fail");

        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

        // Lower nonce: must fail
        let result = send_tx(
            &mut ctx,
            &payer,
            &[set_listing_quote_ix(
                &payer.pubkey(),
                &payer.pubkey(),
                asset_id,
                seller,
                3,
            )],
        )
        .await;
        assert!(result.is_err(), "lower nonce must fail");
    }

    /// Double-init rejection: calling init_registries twice for the same asset_id must fail.
    #[tokio::test]
    async fn init_registries_double_init_fails() {
        let mut ctx = fresh_ctx().await;
        let payer = ctx.payer.insecure_clone();
        bootstrap_config(&mut ctx, &payer).await;
        let (asset_id, rr, rb, pf, pb, mu, ms, pi, sv) = sample_init_args();

        // First init: must succeed
        let ix = init_registries_ix(
            &payer.pubkey(),
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

    #[tokio::test]
    async fn cancel_listing_clears_active_nonce() {
        let mut ctx = fresh_ctx().await;
        let payer = ctx.payer.insecure_clone();
        bootstrap_config(&mut ctx, &payer).await;
        let seller = solana_sdk::signature::Keypair::new();
        let asset_id = Pubkey::new_unique();

        // Fund seller so they can pay for the cancel tx fee
        let lamports = 1_000_000_000;
        let transfer = anchor_lang::solana_program::system_instruction::transfer(
            &payer.pubkey(),
            &seller.pubkey(),
            lamports,
        );
        send_tx(&mut ctx, &payer, &[transfer]).await.unwrap();
        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

        // List
        send_tx(
            &mut ctx,
            &payer,
            &[set_listing_quote_ix(
                &payer.pubkey(),
                &payer.pubkey(),
                asset_id,
                seller.pubkey(),
                7,
            )],
        )
        .await
        .unwrap();
        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

        // Cancel — seller signs
        let (pda, _) = listing_state_pda(&asset_id, &seller.pubkey());
        let cancel = cancel_listing_ix(&seller.pubkey(), pda);
        let recent = ctx.last_blockhash;
        let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
            &[cancel],
            Some(&seller.pubkey()),
            &[&seller],
            recent,
        );
        ctx.banks_client
            .process_transaction(tx)
            .await
            .expect("cancel ok");

        let s = read_listing(&mut ctx, pda).await;
        assert_eq!(s.active_nonce, None);
        assert_eq!(s.last_seen_nonce, 7); // monotonic mark stays
    }

    #[tokio::test]
    async fn cancel_listing_rejects_non_seller() {
        let mut ctx = fresh_ctx().await;
        let payer = ctx.payer.insecure_clone();
        bootstrap_config(&mut ctx, &payer).await;
        let seller = solana_sdk::signature::Keypair::new();
        let imposter = solana_sdk::signature::Keypair::new();
        let asset_id = Pubkey::new_unique();

        // Fund seller and imposter
        for kp in [&seller, &imposter] {
            let t = anchor_lang::solana_program::system_instruction::transfer(
                &payer.pubkey(),
                &kp.pubkey(),
                1_000_000_000,
            );
            send_tx(&mut ctx, &payer, &[t]).await.unwrap();
            ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        }

        send_tx(
            &mut ctx,
            &payer,
            &[set_listing_quote_ix(
                &payer.pubkey(),
                &payer.pubkey(),
                asset_id,
                seller.pubkey(),
                1,
            )],
        )
        .await
        .unwrap();
        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

        // Imposter tries to cancel seller's listing — must fail
        let (pda, _) = listing_state_pda(&asset_id, &seller.pubkey());
        let cancel = cancel_listing_ix(&imposter.pubkey(), pda);
        let recent = ctx.last_blockhash;
        let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
            &[cancel],
            Some(&imposter.pubkey()),
            &[&imposter],
            recent,
        );
        let result = ctx.banks_client.process_transaction(tx).await;
        assert!(result.is_err(), "non-seller cancel must fail");
    }

    // ── Phase 5 tests ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn execute_purchase_happy_path_no_bubblegum() {
        let mut ctx = fresh_ctx().await;
        let payer = ctx.payer.insecure_clone();
        bootstrap_config(&mut ctx, &payer).await;

        // Seller is an ed25519 keypair whose verifying key bytes ARE the Solana pubkey.
        let seller_sk = SigningKey::generate(&mut rand::rngs::OsRng);
        let seller_pk_bytes: [u8; 32] = seller_sk.verifying_key().to_bytes();
        let seller_pubkey = Pubkey::new_from_array(seller_pk_bytes);

        let buyer = Keypair::new();
        let royalty_recipient = Keypair::new();
        let platform_fee_recipient = Keypair::new();

        // Fund all four wallets with 10 SOL each.
        for wallet_pk in [
            &buyer.pubkey(),
            &seller_pubkey,
            &royalty_recipient.pubkey(),
            &platform_fee_recipient.pubkey(),
        ] {
            let t = anchor_lang::solana_program::system_instruction::transfer(
                &payer.pubkey(),
                wallet_pk,
                10_000_000_000,
            );
            send_tx(&mut ctx, &payer, &[t]).await.unwrap();
            ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        }

        // Register the collection (v0.2: required for execute_purchase).
        let collection = Pubkey::new_unique();
        bootstrap_collection(&mut ctx, &payer, collection).await;
        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

        // Init registries: 250 bps royalty, 500 bps platform fee.
        let asset_id = Pubkey::new_unique();
        let init_ix = init_registries_ix(
            &payer.pubkey(),
            &payer.pubkey(),
            asset_id,
            royalty_recipient.pubkey(),
            250,
            platform_fee_recipient.pubkey(),
            500,
            "ar://abc".to_string(),
            [0u8; 32],
            "ern:1.0:01HXY0000000000000000000Y1".to_string(),
            "1.0".to_string(),
        );
        send_tx(&mut ctx, &payer, &[init_ix]).await.unwrap();
        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

        // Set listing quote nonce=1.
        send_tx(
            &mut ctx,
            &payer,
            &[set_listing_quote_ix(
                &payer.pubkey(),
                &payer.pubkey(),
                asset_id,
                seller_pubkey,
                1,
            )],
        )
        .await
        .unwrap();
        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

        // Build SaleOrder + sign it with seller's ed25519 key.
        let now_seconds = ctx
            .banks_client
            .get_sysvar::<solana_sdk::clock::Clock>()
            .await
            .unwrap()
            .unix_timestamp;
        let sale_order = SaleOrder {
            asset_id,
            collection,
            seller_wallet: seller_pubkey,
            price_lamports: 1_000_000_000,
            listing_nonce: 1,
            expires_at: now_seconds + 600,
        };
        let canonical = sale_order.canonical_bytes();
        let sig = seller_sk.sign(&canonical);
        let sig_bytes: [u8; 64] = sig.to_bytes();

        // Two-ix tx: [0] Ed25519 precompile, [1] execute_purchase
        let ed_ix = ed25519_precompile_ix(&seller_pk_bytes, &sig_bytes, &canonical);
        let exec_ix = execute_purchase_ix(
            &buyer.pubkey(),
            &seller_pubkey,
            &royalty_recipient.pubkey(),
            &platform_fee_recipient.pubkey(),
            sale_order,
            0, // ed25519 precompile is at index 0
            BubblegumPlaceholders::default(),
        );

        let recent = ctx.last_blockhash;
        let tx = Transaction::new_signed_with_payer(
            &[ed_ix, exec_ix],
            Some(&buyer.pubkey()),
            &[&buyer],
            recent,
        );
        ctx.banks_client
            .process_transaction(tx)
            .await
            .expect("execute_purchase should succeed");

        // Verify SOL splits.
        // price = 1_000_000_000
        // royalty = 1_000_000_000 * 250 / 10_000 = 25_000_000
        // fee     = 1_000_000_000 * 500 / 10_000 = 50_000_000
        // proceeds= 1_000_000_000 - 25_000_000 - 50_000_000 = 925_000_000
        let seller_bal = ctx.banks_client.get_balance(seller_pubkey).await.unwrap();
        let royalty_bal = ctx
            .banks_client
            .get_balance(royalty_recipient.pubkey())
            .await
            .unwrap();
        let fee_bal = ctx
            .banks_client
            .get_balance(platform_fee_recipient.pubkey())
            .await
            .unwrap();
        assert_eq!(seller_bal, 10_000_000_000 + 925_000_000, "seller proceeds");
        assert_eq!(royalty_bal, 10_000_000_000 + 25_000_000, "royalty");
        assert_eq!(fee_bal, 10_000_000_000 + 50_000_000, "platform fee");

        // Verify listing nonce was cleared.
        let (listing_pda, _) = listing_state_pda(&asset_id, &seller_pubkey);
        let s = read_listing(&mut ctx, listing_pda).await;
        assert_eq!(s.active_nonce, None, "listing nonce must be cleared");
    }

    // Helper: set up the common state needed for execute_purchase rejection tests.
    // Returns (ctx, payer, seller_sk, seller_pubkey, buyer, r_recv, f_recv, asset_id, collection, now_ts)
    async fn setup_for_execute_purchase() -> (
        solana_program_test::ProgramTestContext,
        solana_sdk::signature::Keypair, // payer (cloned)
        SigningKey,
        Pubkey,
        Keypair, // buyer
        Keypair, // royalty_recipient
        Keypair, // platform_fee_recipient
        Pubkey,  // asset_id
        Pubkey,  // collection
        i64,     // unix_timestamp at setup
    ) {
        let mut ctx = fresh_ctx().await;
        let payer = ctx.payer.insecure_clone();
        bootstrap_config(&mut ctx, &payer).await;

        let seller_sk = SigningKey::generate(&mut rand::rngs::OsRng);
        let seller_pubkey = Pubkey::new_from_array(seller_sk.verifying_key().to_bytes());
        let buyer = Keypair::new();
        let r_recv = Keypair::new();
        let f_recv = Keypair::new();

        for wallet_pk in [
            &buyer.pubkey(),
            &seller_pubkey,
            &r_recv.pubkey(),
            &f_recv.pubkey(),
        ] {
            let t = anchor_lang::solana_program::system_instruction::transfer(
                &payer.pubkey(),
                wallet_pk,
                10_000_000_000,
            );
            send_tx(&mut ctx, &payer, &[t]).await.unwrap();
            ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        }

        let collection = Pubkey::new_unique();
        bootstrap_collection(&mut ctx, &payer, collection).await;
        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

        let asset_id = Pubkey::new_unique();
        send_tx(
            &mut ctx,
            &payer,
            &[init_registries_ix(
                &payer.pubkey(),
                &payer.pubkey(),
                asset_id,
                r_recv.pubkey(),
                250,
                f_recv.pubkey(),
                500,
                "ar://x".into(),
                [0u8; 32],
                "ern:1.0:01HXY0000000000000000000Y1".into(),
                "1.0".into(),
            )],
        )
        .await
        .unwrap();
        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

        send_tx(
            &mut ctx,
            &payer,
            &[set_listing_quote_ix(
                &payer.pubkey(),
                &payer.pubkey(),
                asset_id,
                seller_pubkey,
                1,
            )],
        )
        .await
        .unwrap();
        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

        let now_ts = ctx
            .banks_client
            .get_sysvar::<solana_sdk::clock::Clock>()
            .await
            .unwrap()
            .unix_timestamp;

        (
            ctx,
            payer,
            seller_sk,
            seller_pubkey,
            buyer,
            r_recv,
            f_recv,
            asset_id,
            collection,
            now_ts,
        )
    }

    #[tokio::test]
    async fn execute_purchase_rejects_expired_order() {
        let (
            mut ctx,
            _payer,
            seller_sk,
            seller_pubkey,
            buyer,
            r_recv,
            f_recv,
            asset_id,
            collection,
            _now,
        ) = setup_for_execute_purchase().await;

        let order = SaleOrder {
            asset_id,
            collection,
            seller_wallet: seller_pubkey,
            price_lamports: 1_000_000_000,
            listing_nonce: 1,
            expires_at: 1, // way in the past
        };
        let canon = order.canonical_bytes();
        let sig_bytes: [u8; 64] = seller_sk.sign(&canon).to_bytes();
        let pk_bytes: [u8; 32] = seller_sk.verifying_key().to_bytes();

        let ed = ed25519_precompile_ix(&pk_bytes, &sig_bytes, &canon);
        let ex = execute_purchase_ix(
            &buyer.pubkey(),
            &seller_pubkey,
            &r_recv.pubkey(),
            &f_recv.pubkey(),
            order,
            0,
            BubblegumPlaceholders::default(),
        );

        let recent = ctx.last_blockhash;
        let tx =
            Transaction::new_signed_with_payer(&[ed, ex], Some(&buyer.pubkey()), &[&buyer], recent);
        let result = ctx.banks_client.process_transaction(tx).await;
        assert!(result.is_err(), "expired order must fail");
    }

    #[tokio::test]
    async fn execute_purchase_rejects_nonce_mismatch() {
        // active_nonce = 1, SaleOrder.listing_nonce = 999 → ListingNonceMismatch
        let (
            mut ctx,
            _payer,
            seller_sk,
            seller_pubkey,
            buyer,
            r_recv,
            f_recv,
            asset_id,
            collection,
            now_ts,
        ) = setup_for_execute_purchase().await;

        let order = SaleOrder {
            asset_id,
            collection,
            seller_wallet: seller_pubkey,
            price_lamports: 1_000_000_000,
            listing_nonce: 999, // mismatches the active nonce of 1
            expires_at: now_ts + 600,
        };
        let canon = order.canonical_bytes();
        let sig_bytes: [u8; 64] = seller_sk.sign(&canon).to_bytes();
        let pk_bytes: [u8; 32] = seller_sk.verifying_key().to_bytes();

        let ed = ed25519_precompile_ix(&pk_bytes, &sig_bytes, &canon);
        let ex = execute_purchase_ix(
            &buyer.pubkey(),
            &seller_pubkey,
            &r_recv.pubkey(),
            &f_recv.pubkey(),
            order,
            0,
            BubblegumPlaceholders::default(),
        );

        let recent = ctx.last_blockhash;
        let tx =
            Transaction::new_signed_with_payer(&[ed, ex], Some(&buyer.pubkey()), &[&buyer], recent);
        let result = ctx.banks_client.process_transaction(tx).await;
        assert!(result.is_err(), "nonce mismatch must fail");
    }

    #[tokio::test]
    async fn execute_purchase_rejects_wrong_signing_key() {
        // Ed25519 instruction is signed by imposter, but SaleOrder.seller_wallet
        // is the real seller → Ed25519PubkeyMismatch.
        let (
            mut ctx,
            _payer,
            real_seller_sk,
            real_seller_pubkey,
            buyer,
            r_recv,
            f_recv,
            asset_id,
            collection,
            now_ts,
        ) = setup_for_execute_purchase().await;

        let imposter_sk = SigningKey::generate(&mut rand::rngs::OsRng);
        let imposter_pk_bytes: [u8; 32] = imposter_sk.verifying_key().to_bytes();

        let order = SaleOrder {
            asset_id,
            collection,
            seller_wallet: real_seller_pubkey, // claims to be real seller
            price_lamports: 1_000_000_000,
            listing_nonce: 1,
            expires_at: now_ts + 600,
        };
        let canon = order.canonical_bytes();
        // Sign with imposter key, but put imposter's pubkey in the Ed25519 instruction.
        // Our handler will check that the pubkey in the precompile == sale_order.seller_wallet,
        // which will NOT match (imposter != real seller) → Ed25519PubkeyMismatch.
        let sig_bytes: [u8; 64] = imposter_sk.sign(&canon).to_bytes();

        let ed = ed25519_precompile_ix(&imposter_pk_bytes, &sig_bytes, &canon);
        let ex = execute_purchase_ix(
            &buyer.pubkey(),
            &real_seller_pubkey,
            &r_recv.pubkey(),
            &f_recv.pubkey(),
            order,
            0,
            BubblegumPlaceholders::default(),
        );

        let recent = ctx.last_blockhash;
        let tx =
            Transaction::new_signed_with_payer(&[ed, ex], Some(&buyer.pubkey()), &[&buyer], recent);
        let result = ctx.banks_client.process_transaction(tx).await;
        assert!(
            result.is_err(),
            "wrong signing key must fail (Ed25519PubkeyMismatch)"
        );
    }

    // Also verify the real_seller_sk variable is used (suppress unused warning)
    #[allow(dead_code)]
    fn _uses_real_seller_sk(_: SigningKey) {}

    /// Verifies the Purchase event fires on a successful execute_purchase.
    /// Anchor's `emit!` lowers to a `sol_log_data` call which renders as a
    /// `Program data: <base64>` log line; the svc indexer plan parses these.
    #[tokio::test]
    async fn execute_purchase_emits_purchase_event() {
        let mut ctx = fresh_ctx().await;
        let payer = ctx.payer.insecure_clone();
        bootstrap_config(&mut ctx, &payer).await;

        let seller_sk = SigningKey::generate(&mut rand::rngs::OsRng);
        let seller_pk_bytes: [u8; 32] = seller_sk.verifying_key().to_bytes();
        let seller_pubkey = Pubkey::new_from_array(seller_pk_bytes);
        let buyer = Keypair::new();
        let royalty_recipient = Keypair::new();
        let platform_fee_recipient = Keypair::new();

        for wallet_pk in [
            &buyer.pubkey(),
            &seller_pubkey,
            &royalty_recipient.pubkey(),
            &platform_fee_recipient.pubkey(),
        ] {
            let t = anchor_lang::solana_program::system_instruction::transfer(
                &payer.pubkey(),
                wallet_pk,
                10_000_000_000,
            );
            send_tx(&mut ctx, &payer, &[t]).await.unwrap();
            ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        }

        let collection = Pubkey::new_unique();
        bootstrap_collection(&mut ctx, &payer, collection).await;
        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

        let asset_id = Pubkey::new_unique();
        send_tx(
            &mut ctx,
            &payer,
            &[init_registries_ix(
                &payer.pubkey(),
                &payer.pubkey(),
                asset_id,
                royalty_recipient.pubkey(),
                250,
                platform_fee_recipient.pubkey(),
                500,
                "ar://abc".to_string(),
                [0u8; 32],
                "ern:1.0:01HXY0000000000000000000Y2".to_string(),
                "1.0".to_string(),
            )],
        )
        .await
        .unwrap();
        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

        send_tx(
            &mut ctx,
            &payer,
            &[set_listing_quote_ix(
                &payer.pubkey(),
                &payer.pubkey(),
                asset_id,
                seller_pubkey,
                1,
            )],
        )
        .await
        .unwrap();
        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

        let now_seconds = ctx
            .banks_client
            .get_sysvar::<solana_sdk::clock::Clock>()
            .await
            .unwrap()
            .unix_timestamp;
        let sale_order = SaleOrder {
            asset_id,
            collection,
            seller_wallet: seller_pubkey,
            price_lamports: 1_000_000_000,
            listing_nonce: 1,
            expires_at: now_seconds + 600,
        };
        let canonical = sale_order.canonical_bytes();
        let sig_bytes: [u8; 64] = seller_sk.sign(&canonical).to_bytes();

        let ed_ix = ed25519_precompile_ix(&seller_pk_bytes, &sig_bytes, &canonical);
        let exec_ix = execute_purchase_ix(
            &buyer.pubkey(),
            &seller_pubkey,
            &royalty_recipient.pubkey(),
            &platform_fee_recipient.pubkey(),
            sale_order,
            0,
            BubblegumPlaceholders::default(),
        );

        let recent = ctx.last_blockhash;
        let tx = Transaction::new_signed_with_payer(
            &[ed_ix, exec_ix],
            Some(&buyer.pubkey()),
            &[&buyer],
            recent,
        );
        let outcome = ctx
            .banks_client
            .process_transaction_with_metadata(tx)
            .await
            .expect("rpc ok");
        outcome.result.expect("execute_purchase succeeds");
        let logs = outcome.metadata.expect("metadata").log_messages;
        assert!(
            logs.iter().any(|l| l.starts_with("Program data:")),
            "expected a `Program data:` log line carrying the Purchase event; got: {logs:?}"
        );
    }

    /// Admin gate: bootstrap captures payer-as-admin, but init_registries is
    /// signed by a different keypair claiming to be admin. Must fail with
    /// NotAdmin (has_one constraint on ProgramConfig.admin).
    #[tokio::test]
    async fn init_registries_rejects_wrong_admin() {
        let mut ctx = fresh_ctx().await;
        let payer = ctx.payer.insecure_clone();
        bootstrap_config(&mut ctx, &payer).await;

        let imposter = Keypair::new();
        send_tx(
            &mut ctx,
            &payer,
            &[anchor_lang::solana_program::system_instruction::transfer(
                &payer.pubkey(),
                &imposter.pubkey(),
                10_000_000_000,
            )],
        )
        .await
        .unwrap();
        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

        let (asset_id, rr, rb, pf, pb, mu, ms, pi, sv) = sample_init_args();
        let ix = init_registries_ix(
            &imposter.pubkey(),
            &imposter.pubkey(),
            asset_id,
            rr,
            rb,
            pf,
            pb,
            mu,
            ms,
            pi,
            sv,
        );
        let recent = ctx.last_blockhash;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&imposter.pubkey()),
            &[&imposter],
            recent,
        );
        let result = ctx.banks_client.process_transaction(tx).await;
        assert!(
            result.is_err(),
            "init_registries must reject signer that doesn't match ProgramConfig.admin"
        );
    }

    #[tokio::test]
    async fn set_listing_quote_rejects_wrong_admin() {
        let mut ctx = fresh_ctx().await;
        let payer = ctx.payer.insecure_clone();
        bootstrap_config(&mut ctx, &payer).await;

        let imposter = Keypair::new();
        send_tx(
            &mut ctx,
            &payer,
            &[anchor_lang::solana_program::system_instruction::transfer(
                &payer.pubkey(),
                &imposter.pubkey(),
                10_000_000_000,
            )],
        )
        .await
        .unwrap();
        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

        let asset_id = Pubkey::new_unique();
        let seller = Pubkey::new_unique();
        let ix = set_listing_quote_ix(&imposter.pubkey(), &imposter.pubkey(), asset_id, seller, 1);
        let recent = ctx.last_blockhash;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&imposter.pubkey()),
            &[&imposter],
            recent,
        );
        let result = ctx.banks_client.process_transaction(tx).await;
        assert!(
            result.is_err(),
            "set_listing_quote must reject signer that doesn't match ProgramConfig.admin"
        );
    }

    #[tokio::test]
    async fn register_collection_succeeds() {
        use crate::helpers::*;
        use solana_sdk::signer::Signer;
        let mut ctx = fresh_ctx().await;
        let payer = ctx.payer.insecure_clone();
        bootstrap_config(&mut ctx, &payer).await;

        let collection = solana_sdk::pubkey::Pubkey::new_unique();
        let ix = register_collection_ix(&payer.pubkey(), &payer.pubkey(), collection);
        send_tx(&mut ctx, &payer, &[ix])
            .await
            .expect("register_collection");

        // Verify the CollectionRegistry account exists at the expected PDA.
        let (registry_pda, _bump) = collection_registry_pda(&collection);
        let acct = ctx
            .banks_client
            .get_account(registry_pda)
            .await
            .unwrap()
            .expect("CollectionRegistry account missing");
        assert_eq!(acct.owner, eros_marketplace_solana::ID);
    }

    #[tokio::test]
    async fn register_collection_rejects_wrong_admin() {
        use crate::helpers::*;
        use solana_sdk::signer::Signer;
        let mut ctx = fresh_ctx().await;
        let admin = ctx.payer.insecure_clone();
        bootstrap_config(&mut ctx, &admin).await;

        let imposter = solana_sdk::signature::Keypair::new();
        fund(&mut ctx, &imposter.pubkey(), 1_000_000_000).await;

        let collection = solana_sdk::pubkey::Pubkey::new_unique();
        let ix = register_collection_ix(&imposter.pubkey(), &imposter.pubkey(), collection);
        let err = send_tx(&mut ctx, &imposter, &[ix])
            .await
            .expect_err("must fail");
        // NotAdmin is SaleError index 12 → Anchor error code 6012 (0x177c).
        let dbg = format!("{err:?}");
        assert!(
            dbg.contains("6012") || dbg.contains("NotAdmin"),
            "expected NotAdmin (6012), got: {err:?}"
        );
    }

    /// Cross-instruction signature-bypass attack: the Ed25519 instruction is
    /// well-formed and the seller-signed pubkey + message both live inside it,
    /// so the precompile validates successfully. But `message_instruction_index`
    /// is set to 0 instead of u16::MAX. A naive parser (the v0.1.0 one) reads
    /// the message bytes locally to this instruction and accepts whatever the
    /// attacker put there. The hardened parser must reject the descriptor.
    #[tokio::test]
    async fn execute_purchase_rejects_cross_instruction_msg_index() {
        let (
            mut ctx,
            _payer,
            seller_sk,
            seller_pubkey,
            buyer,
            r_recv,
            f_recv,
            asset_id,
            collection,
            now_ts,
        ) = setup_for_execute_purchase().await;

        let order = SaleOrder {
            asset_id,
            collection,
            seller_wallet: seller_pubkey,
            price_lamports: 1_000_000_000,
            listing_nonce: 1,
            expires_at: now_ts + 600,
        };
        let canon = order.canonical_bytes();
        let sig_bytes: [u8; 64] = seller_sk.sign(&canon).to_bytes();
        let pk_bytes: [u8; 32] = seller_sk.verifying_key().to_bytes();

        // msg_ix_index = 0 (not u16::MAX) → must trip Ed25519DescriptorMismatch.
        let ed = ed25519_precompile_ix_with_indices(
            &pk_bytes,
            &sig_bytes,
            &canon,
            u16::MAX, // sig
            u16::MAX, // pk
            0,        // msg — POISONED
        );
        let ex = execute_purchase_ix(
            &buyer.pubkey(),
            &seller_pubkey,
            &r_recv.pubkey(),
            &f_recv.pubkey(),
            order,
            0,
            BubblegumPlaceholders::default(),
        );

        let recent = ctx.last_blockhash;
        let tx =
            Transaction::new_signed_with_payer(&[ed, ex], Some(&buyer.pubkey()), &[&buyer], recent);
        let result = ctx.banks_client.process_transaction(tx).await;
        assert!(
            result.is_err(),
            "cross-instruction msg index must fail (Ed25519DescriptorMismatch)"
        );
    }

    /// Negative: execute_purchase must fail when the collection in SaleOrder has
    /// never been registered via `register_collection`. Anchor will surface this
    /// as AccountNotInitialized (3012) when it tries to load the CollectionRegistry.
    #[tokio::test]
    async fn execute_purchase_rejects_unregistered_collection() {
        let (
            mut ctx,
            _payer,
            seller_sk,
            seller_pubkey,
            buyer,
            r_recv,
            f_recv,
            asset_id,
            _registered_collection, // from setup (but we won't use it)
            now_ts,
        ) = setup_for_execute_purchase().await;

        // Use a brand-new collection pubkey that was never registered.
        let unregistered_collection = Pubkey::new_unique();

        let order = SaleOrder {
            asset_id,
            collection: unregistered_collection,
            seller_wallet: seller_pubkey,
            price_lamports: 1_000_000_000,
            listing_nonce: 1,
            expires_at: now_ts + 600,
        };
        let canon = order.canonical_bytes();
        let sig_bytes: [u8; 64] = seller_sk.sign(&canon).to_bytes();
        let pk_bytes: [u8; 32] = seller_sk.verifying_key().to_bytes();

        let ed = ed25519_precompile_ix(&pk_bytes, &sig_bytes, &canon);
        let ex = execute_purchase_ix(
            &buyer.pubkey(),
            &seller_pubkey,
            &r_recv.pubkey(),
            &f_recv.pubkey(),
            order,
            0,
            BubblegumPlaceholders::default(),
        );

        let recent = ctx.last_blockhash;
        let tx =
            Transaction::new_signed_with_payer(&[ed, ex], Some(&buyer.pubkey()), &[&buyer], recent);
        let result = ctx.banks_client.process_transaction(tx).await;
        // Anchor returns AccountNotInitialized (3012) when a seeds-constrained
        // account has no data at the expected PDA address.
        assert!(
            result.is_err(),
            "unregistered collection must fail (AccountNotInitialized / 3012)"
        );
        let dbg = format!("{result:?}");
        assert!(
            dbg.contains("3012") || dbg.contains("AccountNotInitialized"),
            "expected AccountNotInitialized (3012), got: {dbg}"
        );
    }

    /// Negative: execute_purchase must fail when the `core_collection` account
    /// passed in the instruction does NOT match `SaleOrder.collection`. Anchor's
    /// `address = sale_order.collection` constraint on `core_collection` will
    /// reject with ConstraintAddress (2003).
    #[tokio::test]
    async fn execute_purchase_rejects_collection_account_mismatch() {
        let (
            mut ctx,
            payer,
            seller_sk,
            seller_pubkey,
            buyer,
            r_recv,
            f_recv,
            asset_id,
            collection_a,
            now_ts,
        ) = setup_for_execute_purchase().await;

        // Register a second collection B.
        let collection_b = Pubkey::new_unique();
        bootstrap_collection(&mut ctx, &payer, collection_b).await;
        ctx.last_blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();

        // SaleOrder claims collection = A, but we will swap core_collection to B
        // by constructing the ix with a SaleOrder whose collection field = B, then
        // patching the SaleOrder we sign over.
        //
        // The simplest approach: build an ix with collection_b in SaleOrder but
        // sign over a canonical that uses collection_a. Because the address
        // constraint on core_collection derives from sale_order.collection in the
        // instruction data, passing collection_b as SaleOrder.collection will
        // succeed the core_collection constraint BUT the listing_state PDA is
        // derived from (asset_id, seller_wallet) independent of collection, so
        // the nonce check still runs — the account loaded will match.
        //
        // Easiest way to trigger the mismatch: pass SaleOrder.collection = A to
        // execute_purchase_ix (which sets core_collection account = A), then
        // override the account at the core_collection slot with B's pubkey.
        // We do this by manually patching the AccountMeta list on the instruction.
        let order_with_a = SaleOrder {
            asset_id,
            collection: collection_a,
            seller_wallet: seller_pubkey,
            price_lamports: 1_000_000_000,
            listing_nonce: 1,
            expires_at: now_ts + 600,
        };
        let canon = order_with_a.canonical_bytes();
        let sig_bytes: [u8; 64] = seller_sk.sign(&canon).to_bytes();
        let pk_bytes: [u8; 32] = seller_sk.verifying_key().to_bytes();

        let ed = ed25519_precompile_ix(&pk_bytes, &sig_bytes, &canon);
        let mut ex = execute_purchase_ix(
            &buyer.pubkey(),
            &seller_pubkey,
            &r_recv.pubkey(),
            &f_recv.pubkey(),
            order_with_a,
            0,
            BubblegumPlaceholders::default(),
        );

        // Patch: find the core_collection account slot (the pubkey that equals
        // collection_a) and replace it with collection_b. Anchor will then see
        // that the account key != sale_order.collection and return ConstraintAddress.
        for meta in ex.accounts.iter_mut() {
            if meta.pubkey == collection_a {
                meta.pubkey = collection_b;
                break;
            }
        }

        let recent = ctx.last_blockhash;
        let tx =
            Transaction::new_signed_with_payer(&[ed, ex], Some(&buyer.pubkey()), &[&buyer], recent);
        let result = ctx.banks_client.process_transaction(tx).await;
        // Anchor ConstraintAddress = 2003; CollectionMismatch = SaleError index 15 = 6015.
        assert!(
            result.is_err(),
            "mismatched core_collection account must fail (ConstraintAddress / 2003 or CollectionMismatch / 6015)"
        );
        let dbg = format!("{result:?}");
        assert!(
            dbg.contains("2003") || dbg.contains("ConstraintAddress") || dbg.contains("6015"),
            "expected ConstraintAddress (2003) or CollectionMismatch (6015), got: {dbg}"
        );
    }
}

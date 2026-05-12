/**
 * eros-marketplace-solana integration smoke test.
 *
 * Phase 7 scope: verify the program loads on the local validator and that
 * `init_registries` works end-to-end (creates both PDAs and stores correct
 * values).  The full Bubblegum lifecycle (mint cNFT → delegate → purchase)
 * is deferred to v0.2 — see CHANGELOG.md for context.
 *
 * Run with:
 *   anchor test
 */

import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import { ErosMarketplaceSale } from "../target/types/eros_marketplace_solana";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";

describe("eros-marketplace-solana: smoke", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace
    .ErosMarketplaceSale as Program<ErosMarketplaceSale>;

  // Seed constants (must match seeds.rs in the Rust program).
  const ROYALTY_REGISTRY_SEED = Buffer.from("royalty");
  const MANIFEST_REGISTRY_SEED = Buffer.from("manifest");

  it("init_registries deploys and runs on local validator", async () => {
    // Use a random pubkey as the asset_id (simulates a cNFT asset id).
    const assetId = Keypair.generate().publicKey;
    const royaltyRecv = Keypair.generate().publicKey;
    const platformRecv = Keypair.generate().publicKey;

    const royaltyBps = 250; // 2.5%
    const platformFeeBps = 500; // 5%
    const manifestUri = "ar://testmanifest123";
    const manifestSha256 = Array(32).fill(0) as number[];
    const personaId = "ern:1.0:01HXY0000000000000000000Y1";
    const specVersion = "1.0";

    // Derive the two PDAs that init_registries will create.
    const [royaltyPda] = PublicKey.findProgramAddressSync(
      [ROYALTY_REGISTRY_SEED, assetId.toBuffer()],
      program.programId
    );

    const [manifestPda] = PublicKey.findProgramAddressSync(
      [MANIFEST_REGISTRY_SEED, assetId.toBuffer()],
      program.programId
    );

    // Call init_registries via the Anchor client.
    const txSig = await program.methods
      .initRegistries(
        assetId,
        royaltyRecv,
        royaltyBps,
        platformRecv,
        platformFeeBps,
        manifestUri,
        manifestSha256,
        personaId,
        specVersion
      )
      .rpc();

    console.log("  init_registries tx:", txSig);

    // Fetch and verify the RoyaltyRegistry PDA.
    const royaltyAcct = await program.account.royaltyRegistry.fetch(royaltyPda);
    if (royaltyAcct.royaltyBps !== royaltyBps) {
      throw new Error(
        `royalty_bps mismatch: expected ${royaltyBps}, got ${royaltyAcct.royaltyBps}`
      );
    }
    if (royaltyAcct.platformFeeBps !== platformFeeBps) {
      throw new Error(
        `platform_fee_bps mismatch: expected ${platformFeeBps}, got ${royaltyAcct.platformFeeBps}`
      );
    }
    if (royaltyAcct.royaltyRecipient.toBase58() !== royaltyRecv.toBase58()) {
      throw new Error("royalty_recipient mismatch");
    }
    if (
      royaltyAcct.platformFeeRecipient.toBase58() !== platformRecv.toBase58()
    ) {
      throw new Error("platform_fee_recipient mismatch");
    }

    // Fetch and verify the ManifestRegistry PDA.
    const manifestAcct =
      await program.account.manifestRegistry.fetch(manifestPda);
    if (manifestAcct.manifestUri !== manifestUri) {
      throw new Error(
        `manifest_uri mismatch: expected ${manifestUri}, got ${manifestAcct.manifestUri}`
      );
    }
    if (manifestAcct.personaId !== personaId) {
      throw new Error(
        `persona_id mismatch: expected ${personaId}, got ${manifestAcct.personaId}`
      );
    }
    if (manifestAcct.specVersion !== specVersion) {
      throw new Error(
        `spec_version mismatch: expected ${specVersion}, got ${manifestAcct.specVersion}`
      );
    }

    console.log("  RoyaltyRegistry PDA:", royaltyPda.toBase58());
    console.log("  ManifestRegistry PDA:", manifestPda.toBase58());
    console.log("  All assertions passed.");
  });

  it("init_registries rejects double-init (same asset_id)", async () => {
    const assetId = Keypair.generate().publicKey;
    const royaltyRecv = Keypair.generate().publicKey;
    const platformRecv = Keypair.generate().publicKey;

    // First init should succeed.
    await program.methods
      .initRegistries(
        assetId,
        royaltyRecv,
        250,
        platformRecv,
        500,
        "ar://test",
        Array(32).fill(0) as number[],
        "ern:1.0:01HXY0000000000000000000Y2",
        "1.0"
      )
      .rpc();

    // Second init with the same asset_id must fail (init, not init_if_needed).
    let threw = false;
    try {
      await program.methods
        .initRegistries(
          assetId,
          royaltyRecv,
          250,
          platformRecv,
          500,
          "ar://test",
          Array(32).fill(0) as number[],
          "ern:1.0:01HXY0000000000000000000Y2",
          "1.0"
        )
        .rpc();
    } catch (_e) {
      threw = true;
    }

    if (!threw) {
      throw new Error("Expected double-init to throw, but it did not");
    }

    console.log("  Double-init correctly rejected.");
  });
});

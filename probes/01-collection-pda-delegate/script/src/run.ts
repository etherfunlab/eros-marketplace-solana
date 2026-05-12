/**
 * Probe 01 — Phase 1: validate Bubblegum V2 + mpl-core PermanentTransferDelegate
 * end-to-end on devnet using a WALLET (not PDA) as the delegate.
 *
 * If this succeeds, Phase 2 swaps the wallet delegate for a program-owned PDA
 * via invoke_signed and re-runs the transfer ix.
 */

import { promises as fs } from 'fs';
import path from 'path';
import { createUmi } from '@metaplex-foundation/umi-bundle-defaults';
import {
  generateSigner,
  keypairIdentity,
  none,
  some,
  Umi,
} from '@metaplex-foundation/umi';
import { Keypair as Web3Keypair } from '@solana/web3.js';
import {
  createCollection,
  mplCore,
  fetchCollection,
} from '@metaplex-foundation/mpl-core';
import {
  createTreeV2,
  mintV2,
  transferV2,
  mplBubblegum,
  parseLeafFromMintV2Transaction,
  getAssetWithProof,
} from '@metaplex-foundation/mpl-bubblegum';
import { dasApi } from '@metaplex-foundation/digital-asset-standard-api';

const DEVNET_RPC = process.env.SOLANA_RPC_URL ?? 'https://api.devnet.solana.com';
const HELIUS_KEY = process.env.HELIUS_API_KEY;
const HELIUS_RPC = HELIUS_KEY
  ? `https://devnet.helius-rpc.com/?api-key=${HELIUS_KEY}`
  : null;
const DAS_RPC = HELIUS_RPC ?? DEVNET_RPC;

const RESPONSES_DIR = path.resolve(__dirname, '..', '..', 'helius-responses');

function bigintJsonReplacer(_key: string, value: unknown): unknown {
  return typeof value === 'bigint' ? value.toString() : value;
}

async function dumpJson(name: string, body: unknown): Promise<void> {
  const file = path.join(RESPONSES_DIR, `${name}.json`);
  await fs.writeFile(file, JSON.stringify(body, bigintJsonReplacer, 2));
  console.log(`  → wrote ${file}`);
}

function loadPayer(umi: Umi) {
  const keypairPath = `${process.env.HOME}/.config/solana/id.json`;
  const raw = JSON.parse(require('fs').readFileSync(keypairPath, 'utf8')) as number[];
  const w3 = Web3Keypair.fromSecretKey(Uint8Array.from(raw));
  return umi.eddsa.createKeypairFromSecretKey(w3.secretKey);
}

async function rawDasCall(method: string, params: unknown): Promise<unknown> {
  const res = await fetch(DAS_RPC, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', id: 'probe-01', method, params }),
  });
  return res.json();
}

async function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

// Retry helper for RPC reads that race with tx propagation across the
// Helius load balancer (sendAndConfirm confirms on one node, next read can
// hit a different node that hasn't replicated yet).
async function retry<T>(fn: () => Promise<T>, label: string, tries = 8): Promise<T> {
  let lastErr: unknown;
  for (let i = 0; i < tries; i++) {
    try {
      return await fn();
    } catch (e) {
      lastErr = e;
      const wait = 1500 * (i + 1);
      console.log(`  ${label} attempt ${i + 1}/${tries} failed, retrying in ${wait}ms…`);
      await sleep(wait);
    }
  }
  throw lastErr;
}

async function main(): Promise<void> {
  await fs.mkdir(RESPONSES_DIR, { recursive: true });

  // ── PHASE A ─────────────────────────────────────────────────────────────
  console.log('\n== PHASE A: umi setup ==');
  const umi = createUmi(DEVNET_RPC)
    .use(mplBubblegum())
    .use(mplCore())
    .use(dasApi());
  const payerKp = loadPayer(umi);
  umi.use(keypairIdentity(payerKp));

  const balance = await umi.rpc.getBalance(payerKp.publicKey);
  console.log(`payer: ${payerKp.publicKey}`);
  console.log(`balance: ${Number(balance.basisPoints) / 1e9} SOL`);
  if (Number(balance.basisPoints) < 1_500_000_000) {
    throw new Error(
      `payer balance too low (need >=1.5 SOL on devnet). got ${Number(balance.basisPoints) / 1e9}`,
    );
  }

  const delegateKp = generateSigner(umi);
  const newOwnerKp = generateSigner(umi);
  console.log(`delegate: ${delegateKp.publicKey}`);
  console.log(`new owner: ${newOwnerKp.publicKey}`);

  // ── PHASE B: create Core collection ─────────────────────────────────────
  console.log('\n== PHASE B: create Core collection w/ PermanentTransferDelegate ==');
  const collectionSigner = generateSigner(umi);
  const collectionTx = await createCollection(umi, {
    collection: collectionSigner,
    name: 'Probe01 Collection',
    uri: 'https://example.invalid/probe01.json',
    plugins: [
      {
        type: 'PermanentTransferDelegate',
        authority: { type: 'Address', address: delegateKp.publicKey },
      },
    ],
  }).sendAndConfirm(umi);
  console.log(`collection: ${collectionSigner.publicKey}`);
  console.log(`tx: ${Buffer.from(collectionTx.signature).toString('base64')}`);

  const collectionAcct = await retry(
    () => fetchCollection(umi, collectionSigner.publicKey),
    'fetchCollection',
  );
  await dumpJson('B-collection-account', collectionAcct);
  console.log(`  plugins present: ${(collectionAcct as any).permanentTransferDelegate ? 'PermanentTransferDelegate ✓' : '(none seen)'}`);

  // ── PHASE C: create Bubblegum V2 tree ────────────────────────────────────
  console.log('\n== PHASE C: createTreeV2 ==');
  const merkleTree = generateSigner(umi);
  const createTreeBuilder = await createTreeV2(umi, {
    merkleTree,
    maxDepth: 5,
    maxBufferSize: 8,
    public: false,
  });
  const treeTx = await createTreeBuilder.sendAndConfirm(umi);
  console.log(`tree: ${merkleTree.publicKey}`);
  console.log(`tx: ${Buffer.from(treeTx.signature).toString('base64')}`);

  // ── PHASE D: mint a cNFT into the collection ────────────────────────────
  console.log('\n== PHASE D: mintV2 into collection ==');
  const leafOwnerKp = generateSigner(umi);
  console.log(`leaf owner (initial): ${leafOwnerKp.publicKey}`);
  // mintV2 can hit a stale RPC node that hasn't seen createTreeV2 yet —
  // retry to outlast Helius's load balancer.
  const mintTx = await retry(
    () =>
      mintV2(umi, {
        leafOwner: leafOwnerKp.publicKey,
        merkleTree: merkleTree.publicKey,
        coreCollection: collectionSigner.publicKey,
        metadata: {
          name: 'Probe01 Asset',
          uri: 'https://example.invalid/probe01-asset.json',
          sellerFeeBasisPoints: 0,
          collection: some(collectionSigner.publicKey),
          creators: [],
        },
      }).sendAndConfirm(umi),
    'mintV2',
  );
  console.log(`mint tx: ${Buffer.from(mintTx.signature).toString('base64')}`);

  const leaf = await retry(
    () => parseLeafFromMintV2Transaction(umi, mintTx.signature),
    'parseLeafFromMintV2Transaction',
  );
  console.log(`leaf __kind=${leaf.__kind} id=${leaf.id} nonce=${leaf.nonce}`);
  await dumpJson('D-leaf', leaf);

  // ── PHASE E: confirm asset state via DAS ────────────────────────────────
  console.log('\n== PHASE E: DAS getAsset (initial) ==');
  await sleep(15_000); // give indexers time
  const dasInitial = await rawDasCall('getAsset', { id: leaf.id });
  await dumpJson('E-das-initial', dasInitial);
  console.log(
    `  initial owner per DAS: ${(dasInitial as any)?.result?.ownership?.owner ?? '(none)'}`,
  );

  // ── PHASE F: TransferV2 via permanent delegate ───────────────────────────
  console.log('\n== PHASE F: transferV2 via permanent delegate ==');
  const assetWithProof = await retry(
    () => getAssetWithProof(umi, leaf.id, { truncateCanopy: true }),
    'getAssetWithProof',
  );
  console.log(`asset proof depth: ${assetWithProof.proof.length} nodes`);

  const transferTx = await retry(
    () =>
      transferV2(umi, {
        leafOwner: leafOwnerKp.publicKey,
        newLeafOwner: newOwnerKp.publicKey,
        merkleTree: merkleTree.publicKey,
        coreCollection: collectionSigner.publicKey,
        authority: delegateKp, // ← the test: permanent delegate (NOT leaf owner) signs
        root: assetWithProof.root,
        dataHash: assetWithProof.dataHash,
        creatorHash: assetWithProof.creatorHash,
        nonce: assetWithProof.nonce,
        index: assetWithProof.index,
        proof: assetWithProof.proof,
      }).sendAndConfirm(umi),
    'transferV2',
  );
  console.log(`transfer tx: ${Buffer.from(transferTx.signature).toString('base64')}`);

  // ── PHASE G: confirm new owner via DAS ──────────────────────────────────
  console.log('\n== PHASE G: DAS getAsset (after transfer) ==');
  await sleep(15_000);
  const dasFinal = await rawDasCall('getAsset', { id: leaf.id });
  await dumpJson('G-das-final', dasFinal);
  const finalOwner = (dasFinal as any)?.result?.ownership?.owner;
  console.log(`  final owner per DAS: ${finalOwner}`);
  console.log(`  expected:            ${newOwnerKp.publicKey}`);
  const success = finalOwner === newOwnerKp.publicKey.toString();
  console.log(
    `  ${success ? '✅' : '❌'} ${success ? 'PASS' : 'FAIL'}: owner ${success ? 'updated' : 'NOT updated'}`,
  );

  // ── PHASE H: signatures-for-asset snapshot ──────────────────────────────
  console.log('\n== PHASE H: DAS getSignaturesForAsset ==');
  const sigsForAsset = await rawDasCall('getSignaturesForAsset', { id: leaf.id });
  await dumpJson('H-signatures-for-asset', sigsForAsset);

  // ── SUMMARY ─────────────────────────────────────────────────────────────
  console.log('\n== SUMMARY ==');
  console.log(`Q1 PermanentTransferDelegate accepts wallet pubkey:  ✅ (created OK)`);
  console.log(`Q2 TransferV2 honors permanent delegate signer:      ${success ? '✅' : '❌'}`);
  console.log(`Q3 DAS reflects new owner:                            ${success ? '✅' : '⚠️ check raw'}`);
  console.log(`Q5 core_collection passed in mintV2 (required):      ✅`);
  console.log('');
  console.log(`Tx signatures (base58 via solscan):`);
  const explorer = (sig: Uint8Array) =>
    `https://solscan.io/tx/${require('bs58').default.encode(sig)}?cluster=devnet`;
  console.log(`  collection: ${explorer(collectionTx.signature)}`);
  console.log(`  tree:       ${explorer(treeTx.signature)}`);
  console.log(`  mint:       ${explorer(mintTx.signature)}`);
  console.log(`  transfer:   ${explorer(transferTx.signature)}`);
  console.log('');
  console.log('Phase 2 (PDA-as-delegate via invoke_signed) is next.');
}

main().catch((err) => {
  console.error('\n💥 PROBE FAILED');
  console.error(err);
  process.exit(1);
});

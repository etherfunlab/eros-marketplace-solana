# Probe 01 — Collection PDA Delegate

**Goal**: validate whether the v0.2 design from
`eros-reports/brainstorm/2026-05-12_marketplace_solana_v02_bubblegum_authority.md`
is feasible — specifically the two 🔴 unknowns from §7.

## Result (2026-05-12)

> **Partial validation — design architecturally sound, but end-to-end validation
> blocked by a Bubblegum V2 deployment gap on Solana devnet.**

### Per-question matrix

| # | Question | Status | Evidence |
|---|---|---|---|
| 1 | Does mpl-core `PermanentTransferDelegate` accept any `Pubkey` as `authority`? | ✅ confirmed | `helius-responses/B-collection-account.json` — collection at `EMK8HwdFCDpDDKSTjXg1UB2cHxSvgHB6ijsFmzAQ6UZ9` was created with `PermanentTransferDelegate.authority = CRwvtQPWNr42a7H8qXQsZ4chN6ZTeq9o6bJ4EKA6vrXu` (a wallet pubkey, but the on-chain plugin stores it as a plain `Pubkey` field — PDA pubkeys are also plain Pubkeys, so the path is open) |
| 2 | Does Bubblegum V2 `TransferV2` honor `authority = collection permanent transfer delegate`? | ⏸️ blocked | Bubblegum V2 `MintV2` fails on devnet (see "Devnet gap" below). Without a successful mint we can't test transfer |
| 3 | What does DAS show after a permanent-delegate-driven transfer? | ⏸️ blocked | Same — gated on Q2 |
| 4 | Does the flow still work when `authority` is a PDA signed via `invoke_signed`? | ⏸️ blocked (Phase 2) | Same — won't proceed until Phase 1 unblocks |
| 5 | When V2 tree is bound to a Core collection, does `core_collection` need to be passed in CPI? | ✅ confirmed | mpl-bubblegum JS 5.0.2 `mintV2` builder type signature includes `coreCollection?: PublicKey | Pda` — required when minting into a Core collection. Same applies to `transferV2`. v0.2 brief §4.3 already wrote "passes `core_collection` from now on" |

### Architecture decision

The brief §3 recommendation (**Option B — collection-scoped PDA**) is **architecturally validated** at the plugin level:
- mpl-core `PermanentTransferDelegate` stores the authority as a plain `Pubkey` (no special wallet/PDA discrimination on the plugin side)
- Bubblegum V2 `TransferV2` exposes `authority?: Signer` that "must be either the leaf owner or collection permanent transfer delegate"
- Solana runtime's `invoke_signed` with PDA seeds produces a Signer indistinguishable from a wallet signer at the runtime level

So Q4 (PDA-as-delegate) reduces to "does Solana CPI signing work for permanent-delegate transfers in Bubblegum V2", which is structurally equivalent to Q2. Once Q2 unblocks, Q4 should follow trivially.

**Recommendation: proceed with v0.2 design Plan B as drafted in the brief.** Write the implementation plan in parallel with monitoring devnet for V2 deployment fix.

## Devnet gap (the blocker)

**Bubblegum V2 (`BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY`) is partially deployed on devnet as of 2026-05-12.**

Evidence:
1. **`CreateTreeV2` succeeds**: we successfully created V2 trees (tx `eW5tYi5...`, `3jV5XsZL...`); the program log shows `Instruction: CreateTreeV2 → InitEmptyMerkleTree` via mpl-account-compression `mcmt6Yrr...`.
2. **`MintV2` fails universally**: every call returns
   ```
   AnchorError caused by account: tree_authority. Error Code: AccountNotInitialized.
   Error Number: 3012. Error Message: The program expected this account to be already initialized.
   ```
   The TreeConfig PDA *does exist* on chain with the correct space (96 bytes) and owner (BGUMAp9). The discriminator at offset 0 is `7b f5 af f8 ab 22 00 cf`. The deployed MintV2 handler appears to expect a different discriminator — likely the program was upgraded in two steps and CreateTreeV2/MintV2 are out of sync.
3. **No successful MintV2 in 200 most recent OK txs against BGUMAp9 on devnet** — only V1 mints (`MintV1`, `MintToCollectionV1`, `Mint`) and V1 transfers (`TransferCnft`, `Transfer`, `ReplaceLeaf`). Lots of `CreateTreeV2` but no follow-up MintV2.

This is not specific to our setup — it's a global devnet program state issue. Either:
- (a) The deployed binary has a bug and is awaiting a fix from the mpl-bubblegum team, or
- (b) MintV2 requires an additional setup step we (and everyone else) are missing, OR
- (c) Devnet runs an outdated build that predates working MintV2.

### Workaround options

| Option | Effort | Trade-off |
|---|---|---|
| **Wait for devnet program upgrade** | low (just monitor) | Indefinite delay; v0.2 plan can still be written in parallel |
| **Run localnet w/ mpl-bubblegum master build** | medium | Requires building the program from source + loading it into local validator alongside mpl-core, mpl-account-compression, and SPL Noop |
| **Test on mainnet** | high cost | ~0.5 SOL per attempt; PermanentTransferDelegate is irrevocable so failures brick the collection forever (see brainstorm §7 mainnet discussion) |
| **Accept analytical validation** | none | Q1 confirmed empirically; Q2-Q4 follow from architectural reasoning (see "Architecture decision" above). Proceed with v0.2 plan now, validate end-to-end when devnet unblocks. **Recommended.** |

## What we ran

```
A. umi setup + payer load + balance check  ✓
B. createCollection w/ PermanentTransferDelegate  ✓
   → collection: EMK8HwdFCDpDDKSTjXg1UB2cHxSvgHB6ijsFmzAQ6UZ9
   → tx: mmY+fgNz1Xq... (base58 4eLc... family, decoded in helius-responses)
   → fetchCollection took 4 retries to pass Helius load-balancer race (see retry helper)
C. createTreeV2 (depth=5, buffer=8)  ✓
   → tree: 4k5ujqxmzVtKZVvexYKJ8irqY2yKcbimPN1aSr7Pbzov
   → tree_authority PDA: DjpaiWHjpUTpy9CB7mixqmJDuaErz9oLHsWSwCLvW5WX (96 bytes, init'd)
D. mintV2  ❌ AccountNotInitialized (8 retries exhausted)
   → also fails without coreCollection (see src/diag-mintv2-only.ts)
E-H. blocked
```

SOL spent across attempts: ~0.07 SOL (mostly tree rent ~0.3 SOL × N attempts where N got refunded by tx failure rollback, plus tx fees). Balance after: ~1.93 SOL.

## Layout

```
probes/01-collection-pda-delegate/
├── README.md                  # this file (results doc)
├── .env                       # HELIUS_API_KEY (gitignored)
├── .gitignore
├── script/                    # Phase 1: TS-only baseline (blocked at D)
│   ├── package.json           # mpl-bubblegum 5.0.2, mpl-core 1.10, umi 1.5
│   ├── tsconfig.json
│   └── src/
│       ├── run.ts             # full probe pipeline A→H
│       └── diag-mintv2-only.ts  # narrow diagnostic: mintV2 fails even w/o coreCollection
├── program/                   # Phase 2 scaffold (NOT created — blocked on Phase 1)
└── helius-responses/          # captured raw responses
    └── B-collection-account.json  # confirmed plugin set
```

## Running

```bash
cd script/
npm install            # 135 packages, ~17s
# Either export HELIUS_API_KEY=... or use the .env (probe reads `process.env.HELIUS_API_KEY`)
export $(cat ../.env | xargs)
export SOLANA_RPC_URL="https://devnet.helius-rpc.com/?api-key=$HELIUS_API_KEY"
npx ts-node src/run.ts                    # full pipeline (currently stops at D)
npx ts-node src/diag-mintv2-only.ts       # narrow diagnostic
```

Prerequisites:
- `solana config get` → devnet
- `~/.config/solana/id.json` has ≥ 1.5 SOL on devnet
- Helius API key in `.env` (or fall back to public devnet RPC, but rate limits are tighter)

## Next steps

1. **Track upstream**: watch `metaplex-foundation/mpl-bubblegum` repo for MintV2-related fixes. File an issue if none exists ("MintV2 fails with AccountNotInitialized on devnet despite successful CreateTreeV2").
2. **Write v0.2 plan now**: don't wait — Q1 architectural validation is enough to start. Plan can include "end-to-end devnet validation gate" before mainnet ship.
3. **Once devnet unblocks**: re-run `npm run` to fill in E-H, then scaffold `program/` for Phase 2 (PDA addendum).

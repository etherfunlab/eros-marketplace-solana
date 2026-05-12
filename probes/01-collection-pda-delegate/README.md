# Probe 01 — Collection PDA Delegate

**Goal**: validate whether the v0.2 design from
`eros-reports/brainstorm/2026-05-12_marketplace_solana_v02_bubblegum_authority.md`
is feasible — specifically the two 🔴 unknowns from §7.

## Questions

| # | Question | Phase | Status |
|---|---|---|---|
| 1 | Does mpl-core `PermanentTransferDelegate` accept any `Pubkey` as `authority` (no PDA-specific rejection)? | Phase 1 (wallet baseline) | ⏳ |
| 2 | Does Bubblegum V2 `TransferV2` honor `authority = collection permanent transfer delegate`? | Phase 1 | ⏳ |
| 3 | What does DAS show after a permanent-delegate-driven transfer? (owner update, signatures, event type) | Phase 1 | ⏳ |
| 4 | Does the flow still work when `authority` is a PDA signed via `invoke_signed`? | Phase 2 (PDA addendum) | ⏳ |
| 5 | When V2 tree is bound to a Core collection (mint into collection), does `core_collection` need to be passed in CPI? | Phase 1 | ⏳ |

## Layout

```
probes/01-collection-pda-delegate/
├── README.md                  # this file
├── .gitignore
├── script/                    # Phase 1: TS-only baseline
│   ├── package.json
│   ├── tsconfig.json
│   └── src/run.ts             # single-file probe runner
├── program/                   # Phase 2: tiny Anchor program (filled later)
│   └── (TBD)
└── helius-responses/          # DAS / RPC response snapshots
    └── .gitkeep
```

## Running

```bash
cd script/
yarn install
yarn ts-node src/run.ts
```

Prerequisites:
- `solana config get` points at devnet
- `~/.config/solana/id.json` has at least 2 SOL on devnet
- (optional) `HELIUS_API_KEY` env for richer DAS queries; falls back to public devnet RPC otherwise

## Results

(Filled in once probe runs.)


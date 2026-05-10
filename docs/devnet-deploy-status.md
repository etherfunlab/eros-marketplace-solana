# Devnet deploy status — v0.1.0

**Status:** Not yet deployed.

**Reason:** Solana devnet airdrop faucet is rate-limited; could not obtain
the ~3-5 SOL needed for first deploy at the time of v0.1.0 release.

**To deploy when faucet is available:**

```bash
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
solana airdrop 5 --url devnet                # may need to retry over hours
solana balance --url devnet                  # confirm >= 3 SOL
anchor deploy --provider.cluster devnet
solana program show <program_id> --url devnet
```

Alternative devnet faucets when the CLI airdrop is blocked:
- https://faucet.solana.com/
- https://solfaucet.com/

**Mainnet deploy**: requires audit + multisig. Out of scope for v0.1.0.

**Local-validator testing** (no airdrop needed) works today via
`anchor test` — see README for details.

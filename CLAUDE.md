# March Madness on Seismic

## Rules (MUST FOLLOW)

1. **After every change**, update `README.md` and this `CLAUDE.md` if the change affects documented behavior, architecture, or setup.
2. **Every PR** must include an entry in `docs/changeset.md` describing what was added/changed.
3. **Every prompt** from the user must be saved verbatim to `docs/prompts/<branch-name>/` as a `.txt` file. Filename format: `{timestamp-seconds}-{slug}.txt`. Organize by feature branch name.
4. **When submitting PRs**, write them in the chat for user review. User may leave comments here or on GitHub.
5. **Branch strategy**: Be intentional about what branch you're working off of. Usually `main`, but agents may stack on each other when dependencies exist.
6. **All git branches** must be prefixed with `cdai__` (e.g., `cdai__add-contracts`).
7. **Every task ends with a PR**. After completing work, push the branch and open a PR. GitHub is source of truth — no code goes to main without review.
8. **`scripts/ci.sh` and `.github/workflows/ci.yml` must stay in sync.** If you change one, update the other. The local script mirrors the GitHub workflow exactly so you can validate before pushing.
9. **Run `./scripts/ci.sh` locally before pushing any commits or opening PRs.** CI must pass locally first. No exceptions. If you break CI, fix it before pushing.

## Tech Stack

### Contracts
- **Language**: Seismic Solidity (ssolc) — the only shielded type we use is `sbytes8` for bracket storage. All other data is unshielded (use normal Solidity types).
- **Framework**: sforge (seismic foundry fork) for build, test, deploy
- **Local node**: sanvil (seismic anvil fork)
- **Key pattern**: `sbytes8` values are shielded on-chain; nodes won't reveal underlying values unless contract explicitly exposes them

### TypeScript / Frontend
- **Runtime**: Bun
- **Blockchain client**: seismic-viem (peer dep: viem)
- **React hooks**: seismic-react (peer deps: wagmi, @tanstack/react-query)
- **Auth**: Privy (Twitter, Discord, social logins → embedded wallet)
- **Build**: Vite
- **UI**: React + Tailwind CSS

### Rust (Crates)
- **indexer**: Listens for on-chain events, writes entrant data to JSON
- **server**: Serves indexed data to frontend via HTTP

## Architecture

```
contracts/          — Seismic Solidity smart contracts (sforge project)
packages/
  client/           — TypeScript client library (bracket encoding, contract calls)
  web/              — React frontend (bracket UI, Privy auth)
  tests/            — Integration tests + local dev bracket population
crates/
  indexer/          — Rust event listener + backfill
  server/           — HTTP server for indexed data
data/               — Tournament data (teams, brackets, configs)
docs/               — Technical docs, changeset, prompts
.github/workflows/  — CI: tests, lint, typecheck, build
```

## Contract Interface (MarchMadness.sol)

Key functions:
- `submitBracket(sbytes8 bracket)` — submit shielded bracket, 1 ETH buy-in
- `updateBracket(sbytes8 bracket)` — update bracket before deadline
- `setTag(string tag)` — set/update optional display name (separate from bracket submission)
- `getBracket(address account)` → `bytes8` — before deadline: requires msg.sender == account (signed read); after deadline: anyone can read
- `submitResults(bytes8 results)` — owner only, posts tournament results
- `scoreBracket(address account)` — score a bracket against results (after results posted)
- `collectWinnings()` — winners collect after all brackets scored
- `collectEntryFee()` — refund if contest invalid (28 days after results, not all scored)
- `getEntryCount()` → `uint32` — number of entries (capped at uint32 max with overflow check)

Events:
- `BracketSubmitted(address indexed account)` — emitted on submit AND update

## Bracket Encoding

- 64 bits (bytes8): bit 63 = MSB (sentinel, must be 1), bits 62-0 = 63 game outcomes
- This is identical to jimpo's original bytes8 encoding — no changes needed to his ByteBracket scoring library
- Scoring: jimpo's ByteBracket library (bit-level scoring, max score 192)
- Teams ordered by region, seeded [1,16,8,9,5,12,4,13,6,11,3,14,7,10,2,15] per region

## Shielded Types & Security

- Brackets stored as `sbytes8` (shielded) — hidden until deadline passes
- `getBracket()` is the most security-critical function: MUST validate `msg.sender == account` before deadline
- Use `walletClient.writeContract()` (shielded write) for submissions, NOT `.twriteContract()`
- Use signed reads (`walletClient.readContract()`) to read own bracket before deadline
- After deadline, client should use `.treadContract()` since brackets are publicly readable

## Key Dates
- **Bracket lock**: Wednesday March 18, 2026 at Noon EST (1742313600 unix)
- **No-contest deadline**: 28 days after results posted
- **Entry fee**: 1 ETH (testnet)

## Reference
- Original contract logic: [jimpo/march-madness-dapp](https://github.com/jimpo/march-madness-dapp) — treat his logic as source of truth
- ByteBracket algorithm: by [pursuingpareto](https://gist.github.com/pursuingpareto/b15f1197d96b1a2bbc48)
- Seismic docs: https://docs.seismic.systems
- Fake tournament data: `data/` directory (2026 brackets from ~/code/sports/brackets)

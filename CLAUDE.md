# March Madness on Seismic

## Rules (MUST FOLLOW)

1. **After every change**, update `README.md` and this `CLAUDE.md` if the change affects documented behavior, architecture, or setup.
2. **Every PR** must include an entry in `docs/changeset.md` describing what was added/changed.
3. **Every prompt** from the user must be saved verbatim to `docs/prompts/<branch-name>/` as a `.txt` file. Filename format: `{timestamp-seconds}-{slug}.txt`. Organize by feature branch name.
4. **When submitting PRs**, write them in the chat for user review. User may leave comments here or on GitHub.
5. **Branch strategy**: Be intentional about what branch you're working off of. Usually `main`, but agents may stack on each other when dependencies exist.
6. **All git branches** must be prefixed with `cdai__` (e.g., `cdai__add-contracts`).

## Tech Stack

### Contracts
- **Language**: Seismic Solidity (ssolc) — uses shielded types (`suint256`, `sbool`, `saddress`, `sbytes32`)
- **Framework**: sforge (seismic foundry fork) for build, test, deploy
- **Local node**: sanvil (seismic anvil fork)
- **Key pattern**: `stype` values are shielded on-chain; nodes won't reveal underlying values unless contract explicitly exposes them

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
- `submitBracket(sbytes32 bracket, string tag)` — submit shielded bracket with optional display name, 1 ETH buy-in
- `updateBracket(sbytes32 bracket)` — update bracket before deadline
- `getBracket(address account)` → `bytes32` — before deadline: requires msg.sender == account (signed read); after deadline: anyone can read
- `submitResults(bytes32 results)` — owner only, posts tournament results
- `scoreBracket(address account)` — score a bracket against results (after results posted)
- `collectWinnings()` — winners collect after all brackets scored
- `collectEntryFee()` — refund if contest invalid (28 days after results, not all scored)
- `getEntryCount()` → `uint256` — number of entries

Events:
- `BracketSubmitted(address indexed account)` — emitted on submit AND update

## Bracket Encoding

- 64 bits (bytes8): bit 63 = MSB, bits 62-0 = 63 game outcomes
- **Last byte must equal 1** (sentinel) to distinguish submitted brackets from uninitialized mapping entries
- Scoring: jimpo's ByteBracket library (bit-level scoring, max score 192)
- Teams ordered by region, seeded [1,16,8,9,5,12,4,13,6,11,3,14,7,10,2,15] per region

## Shielded Types & Security

- Brackets stored as `sbytes32` (shielded) — hidden until deadline passes or owner reads
- `getBracket()` is the most security-critical function: MUST validate `msg.sender == account` before deadline
- Use `walletClient.writeContract()` (shielded write) for submissions, NOT `.twriteContract()`
- Use signed reads (`walletClient.readContract()`) to read own bracket before deadline

## Key Dates
- **Bracket lock**: Wednesday March 18, 2026 at Noon EST (1742313600 unix)
- **No-contest deadline**: 28 days after results posted
- **Entry fee**: 1 ETH (testnet)

## Reference
- Original contract logic: [jimpo/march-madness-dapp](https://github.com/jimpo/march-madness-dapp) — treat his logic as source of truth
- Seismic docs: https://docs.seismic.systems
- Fake tournament data: `data/` directory (2026 brackets from ~/code/sports/brackets)

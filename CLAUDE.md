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

> **Keep in sync**: Whenever you add or change anything in this Rules checklist, also update the skill at `.claude/skills/checklist/SKILL.md`.

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
  client/           — TypeScript client library (bracket encoding, scoring, contract calls, types)
  web/              — React frontend (bracket UI, Privy auth, leaderboard, bracket viewer)
  localdev/         — Local dev tools (populate script) + integration tests
crates/
  seismic-march-madness/ — Shared library: types, scoring, simulation, tournament helpers
  kalshi/           — Kalshi odds ingestor (REST + WS + orderbook fetching + edge computation)
  bracket-sim/      — Tournament simulation, calibration (CSV mode + market-making mode)
  indexer/          — Rust event listener + backfill
  server/           — HTTP server for indexed data + tournament status + forecasts
  forecaster/       — Monte Carlo bracket win probability simulator (thin CLI over the lib)
data/               — Tournament data (teams, brackets, configs, tournament-status.json)
docs/               — Technical docs, changeset, prompts
.github/workflows/  — CI: tests, lint, typecheck, build
```

## Contract Interface (MarchMadness.sol)

Constructor: `MarchMadness(uint16 year, uint256 entryFee, uint256 submissionDeadline)`

Key functions:
- `submitBracket(sbytes8 bracket)` — submit shielded bracket, 1 ETH buy-in
- `updateBracket(sbytes8 bracket)` — update bracket before deadline
- `setTag(string tag)` — set/update optional display name (separate from bracket submission)
- `hasEntry(address)` → `bool` — public mapping, true if address has submitted. No signed read needed.
- `getBracket(address account)` → `bytes8` — before deadline: requires msg.sender == account (signed read); after deadline: anyone can read
- `submitResults(bytes8 results)` — owner only, posts tournament results
- `scoreBracket(address account)` — score a bracket against results (after results posted)
- `collectWinnings()` — winners collect after all brackets scored
- `collectEntryFee()` — refund if contest invalid (28 days after results, not all scored)
- `getEntryCount()` → `uint32` — number of entries (capped at uint32 max with overflow check)

Events:
- `BracketSubmitted(address indexed account)` — emitted on submit AND update

## BracketMirror Contract (BracketMirror.sol)

Standalone admin-managed off-chain bracket pool mirror. No money, no scoring, no composition with MarchMadness. All winner computation happens off-chain.

- `createMirror(slug, displayName)` → mirrorId
- `addEntry(mirrorId, bracket, slug)` — admin adds `MirrorEntry { bracket, slug }`; slug must be unique within mirror
- `removeEntry(mirrorId, index)` — swap-and-pop
- `updateBracket(mirrorId, index, bracket)` / `updateEntrySlug(mirrorId, index, slug)`
- `setPrizeDescription(mirrorId, description)` — off-chain prize bookkeeping
- `getEntryBySlug(mirrorId, slug)` — lookup entry by slug for nice URLs (e.g. `/mirrors/mens-league/brackets/my-entry-slug`)
- Entries stored as `MirrorEntry[]` array per mirror
- Existence check: `admin != address(0)` (no `exists` field)

## BracketGroups Contract (BracketGroups.sol)

Linked sub-groups composing with MarchMadness. Optional password + entry fee.

- `createGroup(slug, displayName, entryFee)` → groupId (public)
- `createGroupWithPassword(slug, displayName, entryFee, sbytes12 password)` → groupId (private)
- `joinGroup(groupId, name)` / `joinGroupWithPassword(groupId, sbytes12 password, name)` — payable, always requires name
- `leaveGroup(groupId)` — refund before submission deadline
- `editEntryName(groupId, name)` — update display name
- `scoreEntry(groupId, memberIndex)` — delegates to `marchMadness.scoreBracket()` if not already scored, then reads score
- `collectWinnings(groupId)` — winners split group prize pool after scoring window
- `getGroupBySlug(slug)` → `(uint32, Group memory)` — returns both ID and group data

Password stored as `sbytes12` (shielded). Public groups reject password joins and vice versa.
Group IDs are `uint32`. GroupPayout uses `uint32` for numWinners/numScored.
BracketGroups imports `IMarchMadness` interface (not the full contract) — field named `marchMadness`.
Group struct uses `creator` (not `admin`) since group creators have no extra privileges.
Existence check: `creator != address(0)` (no `exists` field).

## Deploy Scripts

Single deploy script deploys all 3 contracts. BracketGroups receives the MarchMadness address in its constructor.

- **Production**: `contracts/script/DeployAll.s.sol` — deploys MM + Groups + Mirror
- **Local dev**: `contracts/script/DeployAllLocal.s.sol` — same with configurable `DEADLINE_OFFSET`
- **Testnet**: `scripts/deploy-testnet.sh` — runs `DeployAll.s.sol`, writes all 3 addresses to `data/deployments.json`
- **Legacy scripts**: `MarchMadness.s.sol` / `MarchMadnessLocal.s.sol` still work for MM-only deploys

`data/deployments.json` format: `{ "2026": { "5124": { "marchMadness": "0x...", "bracketGroups": "0x...", "bracketMirror": "0x..." } } }`

## Bracket Encoding

- 64 bits (bytes8): bit 63 = MSB (sentinel, must be 1), bits 62-0 = 63 game outcomes
- This is identical to jimpo's original bytes8 encoding — no changes needed to his ByteBracket scoring library
- Scoring: jimpo's ByteBracket library (bit-level scoring, max score 192)
- Teams ordered by region, seeded [1,16,8,9,5,12,4,13,6,11,3,14,7,10,2,15] per region

## Server API

Rust HTTP server (`crates/server`, default port 3000):
- `GET /api/entries` — full entry index (from indexer)
- `GET /api/entries/:address` — single entry by address
- `GET /api/stats` — total entries + scored count
- `GET /api/tournament-status` — tournament status JSON (from `data/tournament-status.json`, TTL cached)
- `POST /api/tournament-status` — update tournament status (requires `Authorization: Bearer <key>`, key set via `TOURNAMENT_API_KEY` env var or `--api-key` flag)
- `GET /api/forecasts` — bracket win probabilities (from `data/forecasts.json`, written by forecaster crate)
- `GET /health` — health check

Frontend env var `VITE_API_BASE` sets the server URL (default `http://localhost:3000`).

Production URL: `https://brackets.seismictest.net/api/...` (nginx proxies `/api/*` to port 3000).
See `docs/api.md` for full API documentation including schema, game index layout, and team names.

## Frontend Routes

- `/` — Home: bracket picker (pre-deadline) or own bracket with tournament overlay (post-deadline)
- `/leaderboard` — All entries ranked by `scoreBracketPartial` (current score, max possible)
- `/bracket/:address` — Read-only bracket view with tournament status overlay

## Shielded Types & Security

- Brackets stored as `sbytes8` (shielded) — hidden until deadline passes
- `getBracket()` is the most security-critical function: MUST validate `msg.sender == account` before deadline
- Use `walletClient.writeContract()` (shielded write) for submissions, NOT `.twriteContract()`
- Use signed reads (`walletClient.readContract()`) to read own bracket before deadline
- After deadline, client should use `.treadContract()` since brackets are publicly readable

## Environment

Single `.env` file at repo root — see `.env.example` for all variables. Never create `.env` files in subdirectories.

- **Vite** loads from root via `envDir: "../../"` in `packages/web/vite.config.ts`
- **Testnet deploy** (`bun deploy:testnet`) sources `.env` for `DEPLOYER_PRIVATE_KEY` and `VITE_RPC_URL`, deploys via sforge, and writes the contract address to `data/deployments.json`
- **Contract address resolution**: `VITE_CONTRACT_ADDRESS` CLI override → `data/deployments.json` (checked-in, keyed by year + chain ID) → zero address fallback
- **Local dev** (populate script) uses hardcoded anvil accounts from `data/anvil-accounts.json` — does not need `DEPLOYER_PRIVATE_KEY`

## Local Development

### Populate Script (`packages/localdev/src/populate.ts`)

Spawns a sanvil node (if not already running), deploys the MarchMadness contract via sforge, populates it with data for the requested phase, then starts the Vite dev server with the contract address injected automatically. Use `--no-vite` to skip starting Vite (e.g. for CI or scripting).

Three phases:
- **`pre-submission`** (default) — deploys contract with future deadline (1 hour). No brackets submitted. Use for testing bracket picker UI and submission flow.
- **`post-submission`** — deploys, submits brackets from anvil test accounts, fast-forwards past deadline, posts results, scores first 3 brackets. Use for testing bracket viewing, scoring UI, off-chain preview. Remaining brackets are left unscored for manual testing.
- **`post-grading`** — full lifecycle: deploy, submit, score all, fast-forward past scoring window. Use for testing payout collection and final leaderboard.

```bash
bun p:pre                     # pre-submission (default)
bun p:post                    # post-submission: brackets + results + partial scoring
bun p:grading                 # post-grading: full lifecycle including payouts
```

Key env vars: `CONTRACT_ADDRESS` (skip deploy), `DEADLINE_OFFSET` (custom deadline), `RPC_URL`.

### Integration Tests (`packages/localdev/test/integration.test.ts`)

Runs against an already-running sanvil node (started externally, e.g. by CI or the populate script). Deploys via sforge, then tests the full contract lifecycle.

```bash
bun run --filter @march-madness/localdev test
```

Tests cover the full contract lifecycle (submit, update, deadline enforcement, scoring, payouts) using the client library against a live sanvil node.

## Key Dates
- **Bracket lock**: Wednesday March 18, 2026 at Noon EST (1773853200 unix)
- **No-contest deadline**: 28 days after results posted
- **Entry fee**: 1 ETH (testnet)

## Seismic RPC Quirks
- **Block timestamps**: Seismic RPC returns **millisecond** timestamps (e.g. via `eth_getBlockByNumber`), but Solidity's `block.timestamp` is still in **seconds**. If you read block timestamps from JS via the RPC, divide by 1000.

## Reference
- Original contract logic: [jimpo/march-madness-dapp](https://github.com/jimpo/march-madness-dapp) — treat his logic as source of truth
- ByteBracket algorithm: by [pursuingpareto](https://gist.github.com/pursuingpareto/b15f1197d96b1a2bbc48)
- Seismic docs: https://docs.seismic.systems
- Fake tournament data: `data/` directory (2026 brackets from ~/code/sports/brackets)

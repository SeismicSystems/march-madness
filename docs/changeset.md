# Changeset

All notable changes to this project. Every PR must add an entry here.

## [Unreleased]

### 2026-03-15 — Mobile header dropdown + better error surfacing
- **Header**: On mobile, replaced inline buttons (Faucet, address, Connect/Disconnect) with a hamburger dropdown menu to prevent text overlap on small screens. Desktop layout unchanged.
- **Error handling**: Added `extractErrorMessage` helper that walks the error cause chain to surface the real error from Privy/viem instead of showing generic "An error has occurred" messages. Errors now show in a scrollable container on mobile.

### 2026-03-14 — Add `/checklist` skill
- Added `.claude/skills/checklist/SKILL.md` — user-invocable skill that mirrors the CLAUDE.md rules checklist, for quick verification before pushing or opening PRs
- Added sync note to CLAUDE.md: any changes to the rules checklist must also be reflected in the skill

### 2026-03-14 — Bracket spacing + sbytes8 fix (seismic-viem PR)
- **Bracket spacing**: Increased R64 vertical gap from 2px to 8px (desktop) / 6px (mobile), and bumped later rounds proportionally. Games no longer appear jammed together.
- **sbytes8 bug**: Root-caused to seismic-viem v1.1.1 missing `sbytes*` in `remapSeismicAbiInputs`. Fix submitted upstream: SeismicSystems/seismic#117 (v1.1.2). Will update dep when published.

### 2026-03-14 — Lazy signed read + hasEntry contract function
- **Contract**: Added `mapping(address => bool) public hasEntry` — set to `true` on `submitBracket()`. Allows anyone to check if an address has submitted without a signed read.
- **ABI + Client**: Added `getHasEntry(address)` to `MarchMadnessPublicClient`
- **Frontend**: On login, calls `hasEntry(address)` (public, no signing) to check submission status. The signed read (`getMyBracket`) is now only triggered when user clicks "Load my bracket" button.
- **SubmitPanel**: Shows "Load my bracket" button when `hasSubmitted` is true but bracket data hasn't been loaded yet
- **Integration test**: Added test for `hasEntry` (true for submitters, false for non-submitters)
- **Redeployed** contract to testnet: `0xD1cA8aDfdaE872D44Af5aACf8a9EfE7493c606cf`

### 2026-03-14 — UX improvements: faucet, localStorage picks, multi-round advancing
- **Faucet link**: added "Faucet" link in header (opens in new tab), plus a prominent `FaucetBanner` when connected with 0 ETH balance — shows address with copy button and "Get Testnet ETH" link
- **Copyable address**: connected wallet address is shown in header and clickable to copy on both mobile and desktop
- **Balance check**: `useContract` now fetches wallet ETH balance; App shows faucet banner when balance is 0
- **LocalStorage picks**: bracket picks persist in `localStorage` keyed by `mm-picks-{address}` (zero address for unauthenticated). On login, zero-address picks migrate to the real address if no existing picks. Picks survive page refresh.
- **Multi-round advancing**: users can now pick a team to advance multiple rounds without filling in their opponent's bracket path. E.g., click Duke → Duke → Duke all the way to the championship. Winner computation allows single-team picks; BracketGame enables clicking a team even when opponent is TBD.
- Added `FAUCET_URL` constant

### 2026-03-14 — Fix deadline timestamp + redeploy contract
- **Bug**: `SUBMISSION_DEADLINE` was `1742313600` (March 18, **2025**) instead of `1773853200` (March 18, **2026**). This caused the app to show "Brackets are locked" a year early.
- Fixed timestamp in `constants.ts` and `MarchMadness.s.sol`
- Redeployed contract to testnet: `0x9cf71ec28D89330fD537b9131752ADA8157622b5`
- Updated `CLAUDE.md` with correct timestamp and documented Seismic RPC millisecond-timestamp quirk
- **Privy connect**: configured Privy app with correct app ID, enabled all social login methods, and added `brackets.seismictest.net` to allowed domains in Privy dashboard

### 2026-03-14 — Mobile-friendly web app (closes #10)
- Added `useIsMobile` hook (viewport < 768px detection via matchMedia)
- BracketView: mobile renders tabbed region selector (East/West/South/Midwest/Final Four) instead of 1400px-wide horizontal layout
- BracketGame: added `mobile` prop — tighter padding (px-1.5, py-0.5), smaller text (11px), smaller min-widths (72-80px) so a full region fits on small screens
- BracketRegion: added `compact` prop — scaled-down vertical spacing between rounds for mobile
- Header: responsive text sizing, hidden entry count badge and address on small screens, shorter "Connect" label
- Scoreboard: responsive padding and font sizes
- App/SubmitPanel: tighter padding on mobile (px-2, p-4)
- Tested down to 320px width (iPhone SE) — each region tab fits without horizontal scroll

### 2026-03-14 — Deploy MarchMadness to Seismic testnet (gcp-2)
- Deployed MarchMadness contract to Seismic testnet (chain 5124): `0xEbc32b5436D7DaA0e5b79431074242a29890364b`
- Entry fee: 1 ETH, submission deadline: March 18, 2026 12:00 PM EST
- Updated `data/deployments.json` with testnet contract address
- Added `contracts/broadcast/` to `.gitignore` (sforge broadcast artifacts)

### 2026-03-14 — Single .env at repo root + testnet deploy script
- Consolidated all env vars into a single `.env` file at repo root (was also in `contracts/.env.example`)
- Added `.env` to root `.gitignore` — the file contains a real testnet deployer private key
- Created `.env.example` with documented placeholders for all env vars (deployment, frontend, local dev)
- Removed `contracts/.env.example` (no longer needed)
- Removed `.env` from `contracts/.gitignore` (root `.gitignore` handles it)
- Added `bun deploy:testnet` script — sources root `.env` for `DEPLOYER_PRIVATE_KEY` and `VITE_RPC_URL` (shared with frontend, no duplicate RPC var), runs sforge with the production deploy script
- Local populate script unchanged — still uses hardcoded anvil accounts, no `DEPLOYER_PRIVATE_KEY` needed
- Contract address resolution: `VITE_CONTRACT_ADDRESS` CLI override → `data/deployments.json` (checked-in, keyed by year + chain ID) → zero address fallback
- Added `data/deployments.json` — source of truth for deployed contract addresses, grouped by year (`{"2026": {"5124": "0x..."}}`). Written automatically by deploy script, easy to extend for 2027+.
- `bun deploy:testnet` runs `scripts/deploy-testnet.sh` — deploys via sforge, parses address, writes to `deployments.json`. Also supports `--contract-address 0x...` to skip deploy and just write the address.
- Populate script starts Vite dev server automatically after deploying, with `VITE_CONTRACT_ADDRESS` and `VITE_CHAIN_ID` injected. Use `--no-vite` to skip.
- Removed `VITE_PUBLIC_RPC_URL` — single `VITE_RPC_URL` used everywhere (wagmi transport + ShieldedWalletProvider public transport)
- Added `target/` to root `.gitignore`
- Updated CLAUDE.md, README.md, docs/technical.md with environment documentation

### 2026-03-14 — PR #8 Review: Restructure tests package to localdev (`packages/localdev`)
- Renamed `packages/tests` to `packages/localdev` (`@march-madness/localdev`) — this is primarily a local dev tool, not just tests
- Moved `integration.test.ts` from `src/` to `test/` directory (at same level as `src/`)
- Added shorthand bun scripts to root `package.json`: `bun p:pre`, `bun p:post`, `bun p:grading`
- Updated all references across CLAUDE.md, README.md, docs/technical.md, packages/mise.toml

### 2026-03-14 — PR #8 Review: Refactor tests to use client library (`packages/tests`)
- Refactored `populate.ts` and `integration.test.ts` to use `MarchMadnessPublicClient`, `MarchMadnessUserClient`, and `MarchMadnessOwnerClient` from `@march-madness/client` instead of raw `wallet.writeContract()` / `publicClient.readContract()` calls
- Added factory functions to `utils.ts`: `createMMPublicClient()`, `createMMUserClient()`, `createMMOwnerClient()`
- Removed local `ENTRY_FEE` constant from `utils.ts` — now re-exported from `@march-madness/client`
- Raw wallet calls kept only where client library cannot express the test (wrong entry fee, cross-user bracket read before deadline, non-owner submitResults)

### 2026-03-14 — Integration Tests & Local Dev Population (`packages/tests`)
- Added `src/utils.ts` — test utilities: random/chalky bracket generation, sforge deploy, sanvil process spawning, anvil account loader, seismic-viem client helpers, time manipulation (evm_increaseTime + evm_mine)
- Added `src/integration.test.ts` — full end-to-end test suite (expects sanvil already running): deploy via sforge, concurrent bracket submission, tags, updates, signed read (own bracket before deadline), fast-forward past deadline, transparent read, results posting, scoring, payout collection with balance verification
- Added `src/populate.ts` — local dev population script that spawns sanvil itself, deploys via sforge, and populates state:
  - `--phase pre-submission` (default): deploy with future deadline, no brackets (for testing submission UI)
  - `--phase post-submission`: deploy, submit all brackets concurrently, fast-forward, post results, score a few (for testing reveal/scoring UI)
  - `--phase post-grading`: everything above + score all + fast-forward past 7-day scoring window (for testing payout UI)
  - Sanvil is left running after the script completes so the frontend can use it
- Added `data/anvil-accounts.json` — all 10 standard anvil accounts with addresses, private keys, and labels
- Added `contracts/.env.example` — deployer key format for sforge script
- Added `tsconfig.json` to tests package, added typecheck/lint/build scripts to `package.json`
- Updated `packages/mise.toml` to include tests package in typecheck, lint, and build tasks

### 2026-03-14 — PR #5 Review Fixes
- provider.rs: Support both SeismicReth (prod) and SeismicFoundry (sanvil) via `IndexerProvider` enum and `--network` CLI flag
- ci.sh: Missing `Cargo.toml` or `cargo` now fails CI instead of silently skipping
- main.rs: Renamed `Check` enum variant to `SanityCheck` (CLI subcommand remains `check` via `#[command(name = "check")]`)

### 2026-03-14 — Rust Indexer Binary (`crates/indexer`)
- Built `march-madness-indexer` — event indexer for MarchMadness contract on Seismic
- Four subcommands via clap: `listen` (live polling), `backfill` (historical scan), `reveal` (post-deadline bracket reading), `check` (sanity check vs on-chain count)
- Uses seismic-alloy provider (`SeismicUnsignedProvider` via `SeismicProviderBuilder`) for all RPC calls
- `sol!` macro for type-safe ABI encoding/decoding of events (`BracketSubmitted`, `TagSet`) and contract calls (`getEntryCount`, `getBracket`)
- Replaced hand-rolled `rpc.rs` (raw reqwest JSON-RPC) with seismic-alloy provider in `provider.rs`
- File-based locking (fs2) for concurrent read/write safety with the server
- Index stored as BTreeMap keyed by lowercase hex address, written as pretty JSON to `data/entries.json`
- Graceful SIGINT shutdown for the listener
- Moved Cargo workspace from `crates/Cargo.toml` to repo root `Cargo.toml`
- Updated CI scripts and GitHub workflow to use root workspace

### 2026-03-14 — PR #6 Review Fixes (`packages/web`)
- Changed address truncation from first 8 + last 8 to first 4 + last 4 chars (e.g., `0x1234...abcd`)
- Replaced Inter font with Fira Mono as the global font (Google Fonts link + CSS body rule)

### 2026-03-14 — Max Privy Login Methods (`packages/web`)
- Expanded loginMethods from [twitter, discord] to all 15 Privy-supported methods: wallet, email, sms, google, twitter, discord, github, linkedin, spotify, instagram, tiktok, apple, farcaster, telegram, passkey
- Removed disableAllExternalWallets restriction to support MetaMask and other external wallets with wallet login method

### 2026-03-14 — Web Frontend Refactor: Use Client Library (`packages/web`)
- Deleted hand-written `src/lib/abi.ts` — ABI now comes from `@march-madness/client`
- Refactored `useContract` hook to use `MarchMadnessPublicClient` (transparent reads) and `MarchMadnessUserClient` (shielded writes, signed reads) from client library
- Replaced manual `walletClient.writeContract()` / `walletClient.readContract()` calls with client library methods (`mmUser.submitBracket()`, `mmUser.getMyBracket()`, etc.)
- Replaced duplicated `ENTRY_FEE` constant with import from `@march-madness/client`; `ENTRY_FEE_DISPLAY` now derived via `formatEther(ENTRY_FEE)`
- Bracket encoding already used `encodeBracket` from client library (no change needed)

### 2026-03-14 — Web Frontend (`packages/web`)
- Built React frontend with full 64-team bracket selection UI
- Added Privy authentication (Twitter, Discord, social logins) with embedded wallet via seismic-react ShieldedWalletProvider
- Bracket UI: 4 regions (East/West/South/Midwest) with visual progression through R64 → R32 → Sweet 16 → Elite 8 → Final Four → Championship
- Click-to-pick interface with automatic downstream clearing when changing picks
- Contract integration via `useContract` hook using `@march-madness/client` library (MarchMadnessPublicClient, MarchMadnessUserClient)
- Submission panel with progress bar (picks/63), entry fee display, tag/name input, encoded bracket preview
- Deadline countdown timer with lock detection (March 18, 2026 noon EST)
- Scoreboard placeholder for post-tournament scoring
- Dark theme with Tailwind CSS v4 (@tailwindcss/vite plugin)
- Env vars: VITE_PRIVY_APP_ID, VITE_CHAIN_ID, VITE_RPC_URL

### 2026-03-14 — Client Library Review Fixes (`packages/client`)
- Replaced hand-written ABI with exact sforge-generated ABI from `contracts/out/MarchMadness.sol/MarchMadness.json` (includes proper `sbytes8` types for shielded inputs)
- Refactored `MarchMadnessPublicClient` to use `getContract()` + `.read.functionName()` pattern (consistent with `UserClient`'s `getShieldedContract` pattern)
- Updated ABI test to verify `sbytes8` type on `submitBracket` and `updateBracket` inputs

### 2026-03-14 — Client Library (`packages/client`)
- Added `src/abi.ts` — MarchMadness contract ABI as const array (uses bytes8 for shielded types, seismic-viem handles shielding)
- Added `src/client.ts` — three-level client hierarchy:
  - `MarchMadnessPublicClient`: transparent reads (entry count, results, deadline, scores, tags)
  - `MarchMadnessUserClient`: shielded writes (submitBracket, updateBracket), signed reads (getMyBracket), transparent writes (setTag, scoreBracket, collectWinnings)
  - `MarchMadnessOwnerClient`: owner-only functions (submitResults)
- Added `src/format.ts` — human-readable bracket formatting (formatBracketLines, formatBracketJSON, getFinalFourSummary, getTeamAdvancements)
- Added `validateBracket(hex)` to `src/bracket.ts` — checks 0x prefix, hex length, and sentinel bit
- Fixed runner-up detection bug in `decodeBracket` — now correctly identifies the Final Four loser
- Updated `src/index.ts` barrel exports for all new modules
- Added tests: `abi.test.ts` (5 tests), `format.test.ts` (7 tests), expanded `bracket.test.ts` (8 new tests for validateBracket + runner-up)
- 25 total tests passing, typecheck clean

### 2026-03-14 — Rust HTTP Server (`crates/server`)
- Built `march-madness-server` HTTP server using axum + tokio
- Endpoints: `GET /api/entries` (full index), `GET /api/entries/:address` (single entry), `GET /api/stats` (total entries + scored count), `GET /health`
- TTL-cached reads of the indexer's JSON file (5s default) with fs2 shared/read file locks
- CORS enabled (Access-Control-Allow-Origin: *) for frontend access
- CLI via clap: `--port` (default 3001) and `--index-file` (default `data/entries.json`)
- Graceful shutdown on SIGINT/SIGTERM
- Structured logging via tracing

### 2026-03-14 — CI Workflows + Local CI Script (mise-based)
- Added `mise.toml` (root) — pins sfoundry (nightly), ssolc (2ebb36d), bun (1.3.9) via mise, mirroring samlaf's setup in the seismic repo
- Added `contracts/mise.toml` — sforge tasks (build, test, fmt-check) with FOUNDRY_SOLC injection
- Added `packages/mise.toml` — bun tasks (typecheck, lint, build, test) for client and web
- Added `.github/workflows/ci.yml` — uses `jdx/mise-action@v2` for contracts and packages, cargo directly for crates, changeset enforcement on PRs
- Added `scripts/ci.sh` — local mirror of GitHub CI using mise, run before pushing
- Initialized crates workspace (common lib + indexer/server bins) and packages workspace (client + web + tests)
- Added CLAUDE.md rules #7 (every task ends with PR), #8 (ci.sh ↔ ci.yml sync), #9 (run CI locally before pushing)

### 2026-03-14 — Smart Contracts
- Added ByteBracket.sol library: ported jimpo's bit-manipulation scoring algorithm to Solidity 0.8 with bytes8 (unchecked blocks for bit ops)
- Added MarchMadness.sol main contract: shielded bracket storage (sbytes8), submit/update/score/payout lifecycle
- 57 tests pass with sforge

### 2026-03-14 — Initial Project Setup
- Created repo structure: contracts/, packages/, crates/, data/, docs/
- Added CLAUDE.md with project rules and architecture
- Added README.md with credits to jimpo and pursuingpareto (ByteBracket algorithm author)
- Tournament data in jimpo's format (name, teams, regions) — data/mens-2026.json
- Saved initial prompts to docs/prompts/

# Changeset

All notable changes to this project. Every PR must add an entry here.

## [Unreleased]

### 2026-03-15 — Make `score_base_bb` public in bracket-sim
- Removed `#[cfg(test)]` and `pub(crate)` gate from `scoring::score_base_bb` so downstream consumers (e.g. the brackets pool-strategy repo) can use it directly instead of duplicating the function.

### 2026-03-15 — Sim: configurable pace dispersion + score-dist calibration tool (closes #41)
- **Generalized pace distribution** in `crates/bracket-sim/src/game.rs` via `Game::sample_count(mean, d)` — a single dispersion ratio `d = variance/mean` controls the distribution family: d<1 uses binomial (underdispersed), d=1 uses Poisson, d>1 uses Gamma-Poisson/NB (overdispersed).
- **Unified regulation and OT paths** — overtime now uses the same pace distribution as regulation instead of the old fixed-pace workaround. The dispersion parameter naturally scales variance with the mean.
- **Calibrated default `DEFAULT_PACE_D = 0.3`** (underdispersed) via score-dist sweep against NCAA tournament empirical targets. At d=0.3 the simulated total-score stddev ≈ 20, closest to the empirical ~19.
- **Panic-free simulation** — all distribution constructors use `match` with deterministic fallbacks instead of `unwrap()`. No panics possible in `sample_count` or `simulate_with_pace`.
- **New CLI binary `score-dist`** — sweeps pace dispersion values and reports game-level statistics (avg total, margin spread, OT frequency, pace stddev) for calibration against empirical data.
- **New CLI flag `--pace-d`** on `sim` binary — overrides the default dispersion ratio.
- **Threaded `pace_d`** through `Tournament` (new field + `with_pace_d()` builder) → `Game::simulate()` → `Game::winner()` → `resolve_overtime()`.
- **New tests**: `sample_count_underdispersed`, `sample_count_overdispersed`, `sample_count_poisson_baseline`, `ot_has_pace_variance`.

### 2026-03-15 — Pipeline orchestration scripts
- **New script** `scripts/refresh.sh` — runs the full KenPom/Kalshi ingestion pipeline (scrape KenPom, fetch raw Kalshi futures, fit anchor model, normalize Kalshi futures, calibrate goose values). Supports `--hours N` flag to control cache TTL (default 6 hours).
- **CI: Python checks** — added `run_python` section to `scripts/ci.sh`: verifies `uv` deps install (`uv sync --frozen`) and runs `scrape_kenpom.py --help` as a smoke test. Wired into `all` and available as `./scripts/ci.sh python`.

### 2026-03-15 — Bracket simulation library and CLI
- **New crate** `crates/bracket-sim` — Poisson-based NCAA tournament simulation engine with Bayesian metric updates. Ported from private `brackets` repo with rand 0.8->0.9 migration for edition 2024 compatibility.
- **Library modules**: team loading/validation, game simulation (Poisson scoring + overtime), tournament orchestration, bracket encoding (ByteBracket u64 format), scoring systems, goose calibration against market odds.
- **CLI binary** `sim` — runs Monte Carlo tournament simulations and prints round-by-round advancement probabilities for all 64 teams.
- **CLI binary** `calibrate` — adjusts team "goose" ratings to match target probabilities (e.g. from Kalshi) using iterative Bayesian calibration with Beta posterior convergence checks.
- **Data files**: `data/2025/tournament.json`, `data/2026/tournament.json`, `data/{year}/kenpom.csv`.

### 2026-03-15 — BracketMirror + BracketGroups contracts
- **New contract** `BracketMirror.sol` — standalone admin-managed off-chain bracket pool mirror. No money, no scoring, no composition with MarchMadness. Entries have unique slugs within a mirror for URL-friendly lookup (`getEntryBySlug`). Swap-and-pop removal.
- **New contract** `BracketGroups.sol` — linked sub-groups composing with MarchMadness via `IMarchMadness` interface. Optional `sbytes12` password protection (shielded), optional entry fee with scoring + payout. Group IDs are `uint32`. Scoring delegates to `marchMadness.scoreBracket()` to avoid double work. Group struct uses `creator` (not `admin`). Join/leave gated by submission deadline.
- **New interface** `IMarchMadness.sol` — minimal 6-function interface (`hasEntry`, `submissionDeadline`, `resultsPostedAt`, `scoreBracket`, `scores`, `isScored`) so BracketGroups only needs the deployed address.
- **Deploy scripts**: `DeployAll.s.sol` (production) and `DeployAllLocal.s.sol` (local dev) deploy all 3 contracts. `deploy-testnet.sh` parses all 3 addresses and writes to `data/deployments.json`.
- **Frontend**: `constants.ts` exports `CONTRACT_ADDRESS`, `GROUPS_CONTRACT_ADDRESS`, `MIRROR_CONTRACT_ADDRESS` from `deployments.json` (handles both old string and new object formats).
- **Tests**: 35 BracketGroups tests (creation, join/leave, password, scoring delegation, payouts, deadline enforcement) + 24 BracketMirror tests (creation, entries, slug lookup, swap-and-pop, access control).
- **MarchMadness constructor**: Added `uint16 year` parameter — contracts are now self-describing for which tournament season they belong to. Deploy scripts pass year (production: `2026`, local: `YEAR` env var, default `2026`).

### 2026-03-15 — Kalshi odds ingestor crate
- **New crate** `crates/kalshi` — standalone Kalshi prediction market odds ingestor for March Madness futures. Fetches round-by-round win probabilities from Kalshi's REST API and WebSocket stream.
- **CLI binary** (`kalshi`) with two subcommands: `fetch` (one-shot REST fetch with file caching) and `watch` (live WebSocket NBBO streaming with periodic CSV writes).
- **Fair value computation**: microprice from order book pressure (bid/ask sizes), with fallback to midpoint. Normalizes probabilities per round, backfills missing teams, and enforces cross-round monotonicity.
- **Team name mapping**: `team_names.toml` maps Kalshi market names to canonical names.
- **Zero dependencies on other workspace crates** — fully standalone, can be used independently of the bracket simulation or forecaster.
- Ported from the `brackets` repo with edition 2024 compatibility fixes (removed explicit `ref` in implicitly-borrowing patterns, collapsed `if` blocks per clippy).

### 2026-03-15 — Python KenPom scripts + UV project
- **New**: `pyproject.toml` — UV Python project (`march-madness-scripts`) with dependencies for data pipeline scripts (cloudscraper, kenpompy, matplotlib, numpy, pandas, scikit-learn).
- **New**: `scripts/scrape_kenpom.py` — scrapes KenPom ratings via kenpompy + cloudscraper (Cloudflare bypass), outputs `data/{YEAR}/kenpom.csv` (team, ortg, drtg, pace). Supports `--bracket-only` filtering and `--seeds-from` bracket CSV.
- **New**: `scripts/fit_kenpom_model.py` — fits per-round logistic regression (degree-2 polynomial features, C=0.1 regularization) from KenPom stats to Kalshi market probabilities. Outputs `data/{YEAR}/kenpom_anchor_model.json` with model coefficients, scaler params, and anchor ranges. Generates fit quality plots to `data/{YEAR}/plots/`.
- **Updated**: `.gitignore` — added `.venv/`, `data/*/plots/`, `__pycache__/`.

### 2026-03-15 — Bracket forecaster: forward Monte Carlo win probabilities
- **New crate** `crates/forecaster` (`march-madness-forecaster`) — reads `data/entries.json` + `data/tournament-status.json` + `data/mens-2026.json`, runs forward Monte Carlo simulations (default 100k) to compute per-bracket win probabilities, writes `data/forecasts.json`.
- **Forward simulation**: resolves games round-by-round. Decided games use known winner, live games use in-game `team1WinProbability`, upcoming games derive P(A beats B) from `teamReachProbabilities` via Bradley-Terry: `P(A wins) = reach[A][r+1] / (reach[A][r+1] + reach[B][r+1])`. Later-round matchups depend on who actually advanced in each simulation — no independent coin-flip approximation.
- **Renamed** `crates/common` → `crates/seismic-march-madness` (`seismic-march-madness` crate). This is the shared library for types, scoring, simulation, and tournament helpers — importable by 3rd-party data providers.
- **Library consolidation**: Moved simulation engine (`simulate.rs`), tournament data loading (`tournament.rs`), and partial scoring helpers from the forecaster into the lib. Forecaster is now a thin CLI wrapper.
- **Library contents**: `scoring.rs` (ByteBracket scoring algorithm), `simulate.rs` (forward Monte Carlo), `tournament.rs` (bracket-order helpers, reach-prob builder, partial scoring), `types.rs` (all shared types).
- **Server**: Added `GET /api/forecasts` endpoint — serves `data/forecasts.json` with TTL cache (same pattern as entries/tournament-status).
- **Client types**: Added `BracketForecast` and `ForecastIndex` TypeScript types.
- **Leaderboard**: When forecasts are available, shows E[Score] and P(Win) columns. Win probability > 10% highlighted in green.
- **Frontend hook**: `useForecasts` — polls `/api/forecasts` every 30s.
- **API docs**: Added `docs/api.md` — full schema documentation with game index layout, all 64 team names, curl examples, Cargo.toml import snippet for the `seismic-march-madness` crate.
- **Server port**: Default port changed from 3001 → 3000 (matches nginx proxy config at `brackets.seismictest.net`).

### 2026-03-15 — Tournament Live UI: leaderboard, bracket viewer, scoring
- **Client library**: Ported ByteBracket scoring algorithm from Solidity to TypeScript BigInt (`scoring.ts`). Added `scoreBracket()` (full), `scoreBracketPartial()` (in-progress with max possible), `getScoringMask()`, `popcount()`, `pairwiseOr()`.
- **Types**: Added shared types in `packages/client/src/types.ts` — `TournamentStatus`, `GameStatus`, `EntryRecord`, `EntryIndex`, `PartialScore`.
- **Rust server**: Added `GET /api/tournament-status` (serves `data/tournament-status.json` with TTL cache) and `POST /api/tournament-status` (API key auth via `TOURNAMENT_API_KEY` env var or `--api-key` flag) for external data sources to push status updates.
- **React Router**: Added `react-router-dom` — routes: `/` (home/bracket picker), `/leaderboard` (scored entries), `/bracket/:address` (read-only viewer).
- **Leaderboard page**: Fetches entries + tournament status, scores each bracket with `scoreBracketPartial`, sorts by score. Shows rank, player tag/address, current/max score, champion pick, and link to bracket viewer.
- **Bracket viewer page**: Read-only bracket view at `/bracket/:address` with tournament status overlay. Breadcrumb nav back to leaderboard.
- **Tournament overlays on BracketGame**: Live games show pulsing green dot + basketball scores + win probability badge. Final correct picks show green checkmark. Final wrong picks show red X + strikethrough + muted opacity.
- **Header nav**: Added Bracket/Leaderboard navigation links (desktop inline, mobile in dropdown menu).
- **Seed data**: Created `data/tournament-status.json` with ~16 R64 finals, ~8 live games, rest upcoming, plus sample `teamReachProbabilities`.
- **Hooks**: `useTournamentStatus` (polls /api/tournament-status every 30s), `useEntries` (polls /api/entries every 30s), `useReadOnlyBracket` (compute GameSlot[] from hex string).

### 2026-03-15 — Desktop bracket layout redesign
- **Bracket convergence**: Fixed double-reversal bug in `BracketRegion` — right-side regions (West, Midwest) now correctly flow inward toward the center where the champion is crowned, matching standard bracket app convention.
- **Submit panel**: Redesigned as a compact horizontal bar on desktop (mobile layout unchanged). Progress, status, entry fee, tag input, and submit button all in one thin row.
- **Removed scoreboard footer**: Removed the placeholder scoreboard section — will revisit later.

### 2026-03-15 — Fix Buffer polyfill for Privy signing
- **Root cause**: Privy's embedded wallet signer calls `Buffer.from()` internally when signing EIP-712 typed data. `Buffer` is a Node.js global not available in browsers.
- Added `buffer` package as devDependency
- Added `Buffer` polyfill in `main.tsx` before any other imports
- Added `global: "globalThis"` to Vite config

### 2026-03-15 — Fix [object Object] in error display
- Error extraction now JSON.stringifies all non-string values so object details render as readable JSON instead of `[object Object]`
- Unrecognized error objects without standard fields are dumped in full

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

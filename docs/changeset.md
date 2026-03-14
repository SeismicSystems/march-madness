# Changeset

All notable changes to this project. Every PR must add an entry here.

## [Unreleased]

### 2026-03-14 — Rust Indexer Binary (`crates/indexer`)
- Built `march-madness-indexer` — event indexer for MarchMadness contract on Seismic
- Four subcommands via clap: `listen` (live polling), `backfill` (historical scan), `reveal` (post-deadline bracket reading), `check` (sanity check vs on-chain count)
- Uses seismic-alloy provider (SeismicUnsignedProvider) for all RPC calls
- sol! macro for type-safe ABI encoding/decoding of events and contract calls
- File-based locking (fs2) for concurrent read/write safety with the server
- Index stored as BTreeMap keyed by lowercase hex address, written as pretty JSON to `data/entries.json`
- Graceful SIGINT shutdown for the listener

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

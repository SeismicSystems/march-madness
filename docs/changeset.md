# Changeset

All notable changes to this project. Every PR must add an entry here.

## [Unreleased]

### 2026-03-14 — CI Workflows + Local CI Script (mise-based)
- Added `mise.toml` (root) — pins sfoundry (nightly), ssolc (2ebb36d), bun (1.3.9) via mise, mirroring samlaf's setup in the seismic repo
- Added `contracts/mise.toml` — sforge tasks (build, test, fmt-check) with FOUNDRY_SOLC injection
- Added `packages/mise.toml` — bun tasks (typecheck, lint, build, test) for client and web
- Added `.github/workflows/ci.yml` — uses `jdx/mise-action@v2` for contracts and packages, cargo directly for crates, changeset enforcement on PRs
- Added `scripts/ci.sh` — local mirror of GitHub CI using mise, run before pushing
- Initialized crates workspace (common lib + indexer/server bins) and packages workspace (client + web + tests)
- Added CLAUDE.md rules #7 (every task ends with PR), #8 (ci.sh ↔ ci.yml sync), #9 (run CI locally before pushing)

### 2026-03-14 — Smart Contracts
- Added ByteBracket.sol library: ported jimpo's bit-manipulation scoring algorithm to Solidity 0.8 with bytes32 (unchecked blocks for bit ops)
- Added MarchMadness.sol main contract: shielded bracket storage (sbytes32), submit/update/score/payout lifecycle
- Added deploy scripts: MarchMadness.s.sol (production, March 18 2026 deadline) and MarchMadnessLocal.s.sol (local dev, 1 hour deadline)
- Added test/jimpo/: ported ByteBracket.js and MarchMadness.js tests to sforge format (8 + 13 tests)
- Added test/slop/: additional tests for Sentinel, AccessControl, Scoring, Payout, NoContest, EdgeCases (44 tests)
- All 65 tests pass with sforge

### 2026-03-14 — Initial Project Setup
- Created repo structure: contracts/, packages/, crates/, data/, docs/
- Added CLAUDE.md with project rules and architecture
- Added README.md with credits to jimpo and pursuingpareto (ByteBracket algorithm author)
- Tournament data in jimpo's format (name, teams, regions) — data/tournament_2026.json
- Removed redundant data files (abbreviations.toml, bracket_config.toml, teams_2026.csv)
- Fixed all types: sbytes8/bytes8 (not sbytes32) — only shielded type in the contract
- Tag submission is a separate function (setTag) from bracket submission
- Entry count uses uint32 with overflow check
- Client should toggle between signed read (before deadline) and transparent read (after)
- Saved initial prompts to docs/prompts/

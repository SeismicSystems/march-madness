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
- Added ByteBracket.sol library: ported jimpo's bit-manipulation scoring algorithm to Solidity 0.8 with bytes8 (unchecked blocks for bit ops)
- Added MarchMadness.sol main contract: shielded bracket storage (sbytes8), submit/update/score/payout lifecycle
- 57 tests pass with sforge

### 2026-03-14 — Initial Project Setup
- Created repo structure: contracts/, packages/, crates/, data/, docs/
- Added CLAUDE.md with project rules and architecture
- Added README.md with credits to jimpo and pursuingpareto (ByteBracket algorithm author)
- Tournament data in jimpo's format (name, teams, regions) — data/mens-2026.json
- Saved initial prompts to docs/prompts/

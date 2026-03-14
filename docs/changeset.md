# Changeset

All notable changes to this project. Every PR must add an entry here.

## [Unreleased]

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

---
"@march-madness/client": minor
"@march-madness/localdev": minor
---

feat: Add tournament results submission tooling

- **compute-results** binary (`crates/ncaa-feed`): Fetches completed bracket from the NCAA API, determines all 63 game outcomes, and encodes results as bytes8 hex for `submitResults(bytes8)`.
- **submit-results** script (`packages/localdev`): TypeScript script using seismic-viem and the client library to preview scores via on-chain `previewScore()`, cross-check with off-chain scoring, display a ranked leaderboard, and submit results with confirmation. Supports `--score-all` to score every bracket after submission.
- **previewScore** added to `MarchMadnessPublicClient` in the client library.
- **ABI regenerated** — now includes `previewScore`, `collectEntryFee`, `RESULTS_DEADLINE`, `hasCollectedEntryFee`.

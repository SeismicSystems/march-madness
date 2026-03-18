---
"@march-madness/web": patch
---

### 2026-03-18 — Add indexer seed command + group leaderboard

- **indexer**: New `seed` subcommand writes fake entries, tournament status, and groups to Redis for local dev. Supports `--entries N` and `--clean` flags.
- **web**: Added `/groups/:slug/leaderboard` and links from joined group cards and public group cards.
- **web**: Group leaderboards now show submitted addresses and tags before reveal, leaving score, max, forecast, champion, and bracket view blank until a revealed bracket exists.
- **web**: Invalid group leaderboard slugs now show an error page instead of falling back to the global leaderboard.
- **web**: Leaderboard-related API polling now uses React Query hooks instead of manual `useEffect` fetch loops.

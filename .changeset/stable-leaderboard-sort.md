---
"@march-madness/web": patch
---

Stabilize leaderboard sort order across data refreshes. Rows no longer reshuffle every 30s when scores/forecasts update — values update in place and order only changes on explicit user sort actions. Also updates default score sort tiebreaker chain to: current score → P(win) → E[score].

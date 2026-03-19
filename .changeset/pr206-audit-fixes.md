---
"march-madness": patch
---

Audit fixes for the multi-pool forecaster follow-up PR

- add `--pre-lock` mode to the forecaster and remove the temporary `--status` override
- keep scoped forecast routes split across `/s/` and `/id/` so slug and ID paths never collide
- ensure the forecaster respects `--tournament-file` consistently and writes deterministic empty forecast/team-prob state

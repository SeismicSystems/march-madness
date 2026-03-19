---
"seismic-march-madness": minor
"ncaa-feed": patch
"march-madness-server": patch
"march-madness-forecaster": patch
"seismic-march-madness-web": patch
---

Clean up tournament status data flow. Remove `teamReachProbabilities` from `TournamentStatus` (now stored separately in `mm:probs`). Server deserializes `mm:games` into a typed `TournamentStatus` before serving. Frontend shows game clock (period + seconds remaining) for live games and derives win probabilities from per-team advance probs via `/team-probs` endpoint.

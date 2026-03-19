---
"march-madness": minor
---

Multi-pool forecaster: Redis-only input, per-pool win probabilities

- Forecaster reads all inputs from Redis (entries, groups, mirrors, tournament status) instead of a JSON file
- Computes per-pool win probabilities: main contest, each group, each mirror
- Stores results as Redis HASH `mm:forecasts` with basis-point values (10000 = 100%)
- Writes per-team advance probabilities to Redis HASH `mm:probs`
- New server endpoints: `/forecasts/groups/s/:slug`, `/forecasts/groups/id/:id`, `/forecasts/mirrors/s/:slug`, `/forecasts/mirrors/id/:id`, `/team-probs`
- Uses rayon for parallel simulation across CPU cores
- Seed command now generates mirrors and team reach probabilities
- Breaking: `mm:forecasts` changed from STRING to HASH; forecast values changed from `BracketForecast` objects to basis-point u32 values

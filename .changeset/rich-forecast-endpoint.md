---
"@march-madness/web": minor
"@march-madness/client": minor
---

Return rich BracketForecast objects from /forecasts endpoint. The forecaster now computes expected score (mean simulated final score) alongside win probability and writes full `{expectedScore, winProbability}` objects to Redis. Frontend consumes these directly — no more bps-to-object transform. Leaderboard labels simplified ("X% / Y pts" instead of "P(Win): X% / E[Score]: Y").

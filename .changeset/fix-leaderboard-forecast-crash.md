---
"@march-madness/web": patch
---

Fix leaderboard crash when forecast data loads. The `/forecasts` API returns `{address: basisPoints}` (plain integers) but the frontend expected `BracketForecast` objects. Now `useForecasts` transforms basis points into proper `BracketForecast` objects, and the E[Score] line is hidden when unavailable.

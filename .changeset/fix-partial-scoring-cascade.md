---
"@march-madness/client": patch
---

Fix live scoring bug where coincidental bit matches on eliminated teams awarded phantom points. The `current` score in `scoreBracketPartial` now applies the same cascade-aware reachability check used for `maxPossible` — a correct bit match only counts if the bracket's predicted team could actually reach that game.

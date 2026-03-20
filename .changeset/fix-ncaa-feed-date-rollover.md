---
"ncaa-feed": patch
---

fix(ncaa-feed): auto-detect date rollover for multi-day tournament polling

The feed previously determined the contest date once at startup and never updated it.
During a multi-day tournament, this meant the feed would keep polling a stale date's
scoreboard and miss all games on subsequent days. Now re-detects the current game day
from the NCAA schedule API whenever the feed is in PreGame phase (no live games),
which naturally covers overnight rollovers and between-day gaps.

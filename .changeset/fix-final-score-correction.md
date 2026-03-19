---
---

**ncaa-feed**: Fix bug where final games never get score corrections. Previously, once a game was marked Final, the feed skipped all further updates — including score corrections from the NCAA API. Now the feed keeps the Final status but still applies score updates when the API reports a different score.

---
---

Fix NCAA scoreboard API response parsing: the API changed `data.scoreboard` to `data.contests` and switched `score`, `seed`, and `startTimeEpoch` from strings to numbers. Also handle ordinal period strings ("1st", "2nd") in addition to numeric ("1", "2").

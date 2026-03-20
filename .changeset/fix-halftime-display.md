---
scope: ncaa-api
type: fix
---

Fix halftime games showing "LIVE" instead of "HALF" in live game banner. At halftime, the NCAA API sends an empty clock string with `currentPeriod: "HALF"` — normalize `clock_seconds` to `Some(0)` so the frontend correctly displays "HALF".
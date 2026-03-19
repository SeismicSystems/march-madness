---
"@march-madness/client": patch
---

Fix live game resolver to use current scores when clock/period data is missing (e.g. during halftime). Previously fell back to pre-game KenPom probability, ignoring the actual score entirely.

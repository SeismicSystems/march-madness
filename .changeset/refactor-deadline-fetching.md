---
"@march-madness/web": minor
---

Refactor deadline fetching to read submission deadline from chain instead of using a hardcoded constant. DeadlineCountdown now shows a loading state until the on-chain value is available.

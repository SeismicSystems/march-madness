---
"@march-madness/web": patch
---

Fix bracket overlay showing teams as "advancing" (green) in rounds they haven't won yet. A team that won R64 was incorrectly highlighted green in R32 before that game was played, because the check used `>= round` (has reached this round) instead of `> round` (has won through this round).

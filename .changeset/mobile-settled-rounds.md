---
"@march-madness/web": patch
---

feat(web): reorder settled rounds to bottom on mobile region views

On mobile, when the tournament is in round N, all previous (settled) rounds are
pushed below the active and future rounds in each region tab. A "Completed Rounds"
divider separates them. The active round is determined globally across all regions —
if any region has a live or final game in round N, all rounds before N are settled
everywhere. Final Four and Live tabs are unaffected.

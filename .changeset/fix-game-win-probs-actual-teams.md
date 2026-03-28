---
"@march-madness/web": patch
---

Fix game win probabilities using actual teams instead of user's bracket picks. When a user's earlier-round pick was wrong, the probability lookup would hit the eliminated team's zero-probability entry, causing the opponent to show 100% win chance. Now uses the actualTeams map for correct team name lookups.

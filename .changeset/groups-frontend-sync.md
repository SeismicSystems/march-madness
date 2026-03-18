---
"@march-madness/web": patch
---

Fix groups UI not updating after join/create/leave: wait for tx receipt, then hydrate group from on-chain data instead of relying on potentially stale API. Also remove inline member list from Your Groups (members are on the group leaderboard).

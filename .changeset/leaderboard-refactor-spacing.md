---
"@march-madness/web": patch
---

Extract shared LeaderboardTable component from LeaderboardPage and MirrorLeaderboardPage, eliminating ~300 lines of duplicated sort/pagination/table rendering logic. Both pages are now thin data-fetching wrappers. Also tighten column spacing on wide screens so numeric columns stay compact and the player name column absorbs extra space.

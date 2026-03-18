---
"@march-madness/web": minor
---

### Leaderboard table → card layout

- **web**: Replaced leaderboard `<table>` with full-width card list using `@fab-ui/card` (shadcn registry). Each entry is a horizontal card with rank, player, champion pick with ESPN team logo, forecast stats, and score.
- **web**: Added shadcn infrastructure (components.json, cn() utility, CSS variables mapped to brand palette).
- **web**: Leaderboard cards are 3/4 width centered on desktop, full-width on mobile. Top-3 entries get brighter gradient backgrounds.

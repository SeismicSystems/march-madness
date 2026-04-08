---
"@march-madness/client": minor
"@march-madness/web": minor
---

feat(web): add Claim Winnings UI to leaderboard

After the scoring window closes, winners can now claim their prize directly from the leaderboard. A WinningsBanner appears above the table with a "Claim Winnings" button showing the payout amount. Per-group leaderboards support the same flow via `collectWinnings(groupId)`, plus a "Score All Members" button for unscored groups. The no-contest escape hatch (`collectEntryFee`) is also surfaced when the owner never posts results within 90 days.

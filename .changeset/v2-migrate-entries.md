---
"@march-madness/localdev": patch
---

Add `migrate-entries.ts` script for full V1 → V2 migration: enumerates entries from on-chain `BracketSubmitted` events, reverses legacy bracket bit encoding, batch-imports into MarchMadnessV2, and migrates BracketGroups + members via BracketGroupsV2. MM addresses are derived from the BG contracts — only `--old-bg` and `--new-bg` are required. Adds `bun run migrate:entries` shortcut at root.

---
"@march-madness/localdev": patch
---

Add `march-madness-populate` Rust binary for migrating Redis data into MarchMadnessV2 and BracketGroupsV2 contracts.

- Reads entries, tags, and groups from Redis
- Converts legacy-encoded brackets to contract-correct encoding via `reverse_game_bits()`
- Batch-imports into V2 contracts using `batchImportEntries`, `importTag`, `importGroup`, `batchImportMembers`
- Idempotent: checks `hasEntry()` on-chain before importing, safe to restart
- Supports `--dry-run`, `--batch-size`, `--skip-entries`, `--skip-groups`
- Uses actual V2 contract signatures from PR #279

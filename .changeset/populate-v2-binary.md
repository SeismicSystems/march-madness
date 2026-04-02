---
"@march-madness/localdev": patch
---

Add `march-madness-populate` Rust binary for migrating V1 contract data into MarchMadnessV2 and BracketGroupsV2 contracts.

- Reads entries, tags, groups, and member names from V1 contracts (events for discovery, view functions for data)
- Converts legacy-encoded brackets to contract-correct encoding via `reverse_game_bits()`
- Batch-imports into V2 contracts using `batchImportEntries`, `importTag`, `importGroup`, `batchImportMembers`
- Idempotent: Redis SET keys track migration progress, V2 batch functions skip already-imported items
- Supports `--dry-run`, `--batch-size`, `--skip-entries`, `--skip-groups`
- Uses actual V2 contract signatures from PR #279

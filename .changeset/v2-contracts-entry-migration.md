---
"@march-madness/client": patch
"@march-madness/localdev": patch
---

Add MarchMadnessV2 and BracketGroupsV2 contracts with owner-only import surface for the legacy encoding migration (steps 3 & 4 of #251).

**Contracts:**

- `MarchMadnessV2` inherits V1, adds `importEntry`, `batchImportEntries`, `importTag`, `fund()`, `receive()`, and `previewScore` (non-mutating scoring preview against arbitrary candidate results)
- `BracketGroupsV2` inherits V1, adds `owner`, `importGroup`, `importMember`, `batchImportMembers`, `fund()`, `receive()`
- `MarchMadness.sol`: `brackets` mapping changed `private → internal` to enable V2 inheritance; `getBracket` marked `virtual`

**Scripts:**

- `scripts/deploy-v2.sh` / `bun deploy:v2` — deploy both V2 contracts to testnet
- `packages/localdev/src/migrate-entries.ts` / `bun migrate:entries` — snapshot V1 brackets, apply 63-bit reversal, batch-import into V2, write manifest JSON

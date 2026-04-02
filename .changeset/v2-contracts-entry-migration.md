---
"@march-madness/client": patch
---

Add MarchMadnessV2 and BracketGroupsV2 contracts with owner-only import surface for the legacy encoding migration (steps 3 & 4 of #251).

**Contracts:**

- `MarchMadnessV2` inherits V1, adds `importEntry` (payable, validates `msg.value == entryFee`), `batchImportEntries` (payable, validates `msg.value == accounts.length * entryFee`), and `importTag`; removes `fund()`/`receive()` (fees paid inline)
- `BracketGroupsV2` inherits V1, adds `owner`, `importGroup`, `importMember` (payable, validates `msg.value == entryFee`), `batchImportMembers` (payable, validates `msg.value == addrs.length * entryFee`); removes `fund()`/`receive()` (fees paid inline)
- `MarchMadness.sol`: `brackets` mapping changed `private → internal` to enable V2 inheritance; `getBracket` marked `virtual`; `previewScore` moved here (it has no relation to migration permissions)

**Scripts:**

- `scripts/deploy-v2.sh` / `bun deploy:v2` — deploy both V2 contracts to testnet

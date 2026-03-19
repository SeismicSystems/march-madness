---
"@march-madness/localdev": patch
---

Add Yahoo Fantasy bracket mirror importer pipeline: Rust binary fetches Yahoo API data and encodes brackets to bytes8 hex (`mirror-importer` crate), Bun script creates/updates BracketMirror entries on-chain, shell wrapper orchestrates both steps. Includes response caching, Yahoo-to-NCAA team name mappings, and idempotent on-chain mirroring.

---
---

fix(rust): correct bracket encoding to match Solidity contract bit ordering

Changed Rust bracket encoding from legacy (game 0 → bit 62) to contract-correct (game 0 → bit 0), matching how Solidity's ByteBracket.getBracketScore processes bits. Updated all Rust code that maps game indices to bit positions: bracket-sim encoding/decoding, seismic-march-madness scoring helpers and simulation callbacks, forecaster, and mirror-importer. Added comprehensive tests for contract-correct round values, encoding parity, and jimpo's Solidity test vectors.

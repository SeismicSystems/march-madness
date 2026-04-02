---
"@march-madness/client": minor
"@march-madness/web": minor
---

fix(client,web): correct TypeScript bracket encoding to match Solidity ByteBracket bit layout

The TypeScript bracket encoding was reversed relative to what the Solidity ByteBracket scoring loop expects. `picks[i]` was mapped to bit `62 - i` but should be mapped to bit `i` (bit 0 = first R64 game, bit 62 = championship).

Changes:
- Fix `encodeBracket`/`decodeBracket` to use contract-correct bit layout (`picks[i]` → bit `i`)
- Fix `scoreBracketPartial` pick extraction to match new encoding
- Fix `useBracket` and `useReadOnlyBracket` hooks to decode bits correctly
- Add `reverseGameBits` utility for one-time localStorage migration
- Add localStorage migration (`mm-encoding-v` version flag) to convert legacy hex values
- Update golden test vectors to contract-correct encoding
- Add comprehensive encoding correctness tests (bit position, Solidity cross-validation, reverseGameBits)

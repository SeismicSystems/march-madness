//! ByteBracket scoring — Rust port of contracts/src/ByteBracket.sol.
//! Uses u64 for bit manipulation, identical logic to Solidity.

const SENTINEL_BIT: u64 = 1u64 << 63;

/// Count the number of 1-bits in a 64-bit value.
pub fn popcount(mut bits: u64) -> u8 {
    bits -= (bits >> 1) & 0x5555_5555_5555_5555;
    bits = (bits & 0x3333_3333_3333_3333) + ((bits >> 2) & 0x3333_3333_3333_3333);
    bits = (bits.wrapping_add(bits >> 4)) & 0x0F0F_0F0F_0F0F_0F0F;
    (bits.wrapping_mul(0x0101_0101_0101_0101) >> 56) as u8
}

/// Pairwise OR — takes bits two at a time and ORs them, producing half-length.
pub fn pairwise_or(mut bits: u64) -> u64 {
    let mut tmp;
    tmp = (bits ^ (bits >> 1)) & 0x2222_2222;
    bits ^= tmp ^ (tmp << 1);
    tmp = (bits ^ (bits >> 2)) & 0x0C0C_0C0C;
    bits ^= tmp ^ (tmp << 2);
    tmp = (bits ^ (bits >> 4)) & 0x00F0_00F0;
    bits ^= tmp ^ (tmp << 4);
    tmp = (bits ^ (bits >> 8)) & 0x0000_FF00;
    bits ^= tmp ^ (tmp << 8);
    let evens = bits >> 16;
    let odds = bits & 0xFFFF;
    evens | odds
}

/// Compute the 64-bit scoring mask from results bits.
pub fn get_scoring_mask(results: u64) -> u64 {
    let mut r = results;
    let mut mask: u64 = 0;

    // Filter for bit 62 (second MSB)
    let bit_selector: u64 = 0x4000_0000_0000_0000;
    for _ in 0..31 {
        mask <<= 2;
        if r & bit_selector != 0 {
            mask |= 1;
        } else {
            mask |= 2;
        }
        r <<= 1;
    }
    mask
}

/// Reverse the 63 non-sentinel game bits while preserving the sentinel bit.
///
/// Rust and TypeScript currently store brackets in a legacy logical game order
/// where `game 0 -> bit 62` and `game 62 -> bit 0`. The exact Solidity
/// `ByteBracket` full scorer consumes the same raw `bytes8` value but scores it
/// in the opposite 63-bit game order. This helper is the compatibility shim
/// between those two interpretations.
pub fn reverse_game_bits(bb: u64) -> u64 {
    let mut out = bb & SENTINEL_BIT;
    for i in 0..63u32 {
        if (bb >> i) & 1 == 1 {
            out |= 1u64 << (62 - i);
        }
    }
    out
}

/// Score a bracket against results (full tournament). Max 192.
///
/// Exact Rust port of `ByteBracket.getBracketScore` from
/// `contracts/src/ByteBracket.sol`.
///
/// Both `bracket` and `results` must be in contract-correct encoding
/// (game 0 → bit 0, game 62 → bit 62). This is the canonical encoding
/// used everywhere in the Rust codebase. For legacy-encoded brackets,
/// use [`score_bracket_legacy`].
pub fn score_bracket(bracket: u64, results: u64) -> u32 {
    let filter = get_scoring_mask(results);
    score_bracket_with_mask(bracket, results, filter)
}

/// Score a legacy off-chain bracket/results pair by first converting the 63
/// game bits into the exact Solidity ByteBracket ordering.
pub fn score_bracket_legacy(bracket: u64, results: u64) -> u32 {
    score_bracket(reverse_game_bits(bracket), reverse_game_bits(results))
}

/// Score with a precomputed mask (for batch scoring).
pub fn score_bracket_with_mask(bracket: u64, results: u64, mut filter: u64) -> u32 {
    let mut points: u32 = 0;
    let mut round_num: u32 = 0;
    let mut num_games: u32 = 32;
    let mut blacklist: u64 = (1u64 << num_games) - 1;
    let mut overlap: u64 = !(bracket ^ results);

    while num_games > 0 {
        let scores = overlap & blacklist;
        points += (popcount(scores) as u32) << round_num;
        blacklist = pairwise_or(scores & filter);
        overlap >>= num_games;
        filter >>= num_games;
        num_games /= 2;
        round_num += 1;
    }

    points
}

/// Parse a hex string (0x-prefixed or bare) into u64 bracket bits.
pub fn parse_bracket_hex(hex: &str) -> Option<u64> {
    let stripped = hex.strip_prefix("0x").unwrap_or(hex);
    u64::from_str_radix(stripped, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_popcount() {
        assert_eq!(popcount(0), 0);
        assert_eq!(popcount(0xFFFF_FFFF_FFFF_FFFF), 64);
        assert_eq!(popcount(1), 1);
        assert_eq!(popcount(0xFF), 8);
    }

    #[test]
    fn test_perfect_bracket() {
        // All chalk (sentinel + all 63 game bits set)
        let chalky = 0xFFFF_FFFF_FFFF_FFFFu64;
        assert_eq!(score_bracket(chalky, chalky), 192);
    }

    #[test]
    fn test_completely_wrong() {
        // All team1 wins vs all team2 wins
        let all_team1 = 0xFFFF_FFFF_FFFF_FFFFu64;
        let all_team2 = 0x8000_0000_0000_0000u64;
        assert_eq!(score_bracket(all_team1, all_team2), 0);
    }

    #[test]
    fn test_parse_bracket_hex() {
        assert_eq!(
            parse_bracket_hex("0xfffffffffffffffe"),
            Some(0xFFFF_FFFF_FFFF_FFFEu64)
        );
        assert_eq!(
            parse_bracket_hex("8000000000000000"),
            Some(0x8000_0000_0000_0000u64)
        );
        assert_eq!(parse_bracket_hex("nope"), None);
    }

    #[test]
    fn reverse_game_bits_preserves_sentinel() {
        assert_eq!(
            reverse_game_bits(0x8000_0000_0000_0000),
            0x8000_0000_0000_0000
        );
    }

    #[test]
    fn reverse_game_bits_matches_known_contract_example() {
        // Jimpo contract example in exact Solidity ordering.
        let contract_bracket = 0xC000_0000_0000_0000u64;
        let contract_results = 0x8000_0000_0000_0000u64;
        assert_eq!(score_bracket(contract_bracket, contract_results), 160);

        // The legacy off-chain representation is the 63-bit reversal of that
        // contract encoding. The compatibility shim should recover the same score.
        let legacy_bracket = reverse_game_bits(contract_bracket);
        let legacy_results = reverse_game_bits(contract_results);
        assert_eq!(legacy_bracket, 0x8000_0000_0000_0001u64);
        assert_eq!(score_bracket_legacy(legacy_bracket, legacy_results), 160);
    }

    // ── Golden vector tests (cross-language consistency) ────────────────

    /// Load golden test vectors from data/test-vectors/bracket-vectors.json.
    /// These vectors are the source of truth, shared with TypeScript and Solidity.
    fn load_vectors() -> serde_json::Value {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/test-vectors/bracket-vectors.json"
        );
        let data = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Failed to read test vectors at {}: {}", path, e));
        serde_json::from_str(&data).expect("Failed to parse test vectors JSON")
    }

    #[test]
    fn golden_vectors_encoding_parity() {
        // The JSON test vectors store hex in legacy encoding (game 0 → bit 62).
        // Verify that contract-correct encoding (game 0 → bit 0) of the same picks
        // produces the reverse of the legacy hex.
        let vectors = load_vectors();
        let brackets = vectors["brackets"].as_array().unwrap();

        for v in brackets {
            let name = v["name"].as_str().unwrap();
            let legacy_hex = v["hex"].as_str().unwrap();
            let picks = v["picks"].as_array().unwrap();

            // Encode picks in contract-correct order (game 0 → bit 0)
            let mut contract_bits: u64 = 1u64 << 63; // sentinel
            for (i, pick) in picks.iter().enumerate() {
                if pick.as_bool().unwrap() {
                    contract_bits |= 1u64 << i;
                }
            }

            let legacy_bits = parse_bracket_hex(legacy_hex).unwrap();
            assert_eq!(
                reverse_game_bits(contract_bits),
                legacy_bits,
                "Encoding parity failed for '{}': reverse(contract) != legacy",
                name
            );
            assert_eq!(
                reverse_game_bits(legacy_bits),
                contract_bits,
                "Encoding parity failed for '{}': reverse(legacy) != contract",
                name
            );
        }
    }

    #[test]
    fn golden_vectors_scoring() {
        // The JSON expected scores are computed against legacy-encoded hex.
        // The scoring function is encoding-dependent (getScoringMask assigns
        // different round values based on bit position), so legacy and
        // contract-correct scores differ for non-trivial cases.
        let vectors = load_vectors();
        let scoring_tests = vectors["scoringTests"].as_array().unwrap();

        for st in scoring_tests {
            let description = st["description"].as_str().unwrap();
            let bracket_hex = st["bracket"].as_str().unwrap();
            let results_hex = st["results"].as_str().unwrap();
            let expected_score = st["expectedScore"].as_u64().unwrap() as u32;

            let legacy_bracket = parse_bracket_hex(bracket_hex).unwrap();
            let legacy_results = parse_bracket_hex(results_hex).unwrap();

            // Legacy scoring should match JSON expected scores
            let legacy_score = score_bracket(legacy_bracket, legacy_results);
            assert_eq!(
                legacy_score, expected_score,
                "Legacy scoring mismatch for '{}': bracket={}, results={}",
                description, bracket_hex, results_hex
            );

            // score_bracket_legacy reverses to contract-correct first.
            // Verify it produces a consistent score (same as direct contract-correct).
            let contract_bracket = reverse_game_bits(legacy_bracket);
            let contract_results = reverse_game_bits(legacy_results);
            let contract_score = score_bracket(contract_bracket, contract_results);
            let legacy_api_score = score_bracket_legacy(legacy_bracket, legacy_results);
            assert_eq!(
                legacy_api_score, contract_score,
                "score_bracket_legacy should equal direct contract-correct scoring for '{}'",
                description
            );
        }
    }

    #[test]
    fn golden_vectors_self_score_192() {
        let vectors = load_vectors();
        let brackets = vectors["brackets"].as_array().unwrap();

        for v in brackets {
            let name = v["name"].as_str().unwrap();
            let hex = v["hex"].as_str().unwrap();
            let legacy_bits = parse_bracket_hex(hex).unwrap();

            // Self-score in legacy encoding
            assert_eq!(
                score_bracket(legacy_bits, legacy_bits),
                192,
                "Legacy self-score should be 192 for '{}'",
                name
            );

            // Self-score in contract-correct encoding
            let contract_bits = reverse_game_bits(legacy_bits);
            assert_eq!(
                score_bracket(contract_bits, contract_bits),
                192,
                "Contract-correct self-score should be 192 for '{}'",
                name
            );
        }
    }

    #[test]
    fn golden_vectors_validation() {
        let vectors = load_vectors();
        let validation_tests = vectors["validationTests"].as_array().unwrap();

        for vt in validation_tests {
            let hex = vt["hex"].as_str().unwrap();
            let expected_valid = vt["valid"].as_bool().unwrap();
            let reason = vt["reason"].as_str().unwrap();

            if let Some(bits) = parse_bracket_hex(hex) {
                let has_sentinel = (bits >> 63) & 1 == 1;
                assert_eq!(
                    has_sentinel, expected_valid,
                    "Validation mismatch for '{}' ({}): expected valid={}",
                    hex, reason, expected_valid
                );
            } else {
                assert!(
                    !expected_valid,
                    "Parse failed for '{}' but expected valid",
                    hex
                );
            }
        }
    }

    // ── Contract-correct encoding tests ────────────────────────────────

    #[test]
    fn test_jimpo_contract_vectors() {
        // These vectors come directly from jimpo's Solidity tests.
        // They are in contract-correct encoding (the format the chain actually uses).
        assert_eq!(
            score_bracket(0xFFFF_FFFF_FFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFF),
            192
        );
        assert_eq!(
            score_bracket(0xC000_0000_0000_0000, 0x8000_0000_0000_0000),
            160
        );
        assert_eq!(
            score_bracket(0x8000_0000_FFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFF),
            32
        );
        assert_eq!(
            score_bracket(0xFFFF_5555_FFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFF),
            176
        );
        assert_eq!(
            score_bracket(0xFFFF_AAAA_FFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFF),
            48
        );
    }

    #[test]
    fn test_contract_correct_round_values() {
        // All chalk bracket and results
        let perfect = 0xFFFF_FFFF_FFFF_FFFFu64;

        // Flip bit 0 (game 0, R64) — should lose exactly 1 point (R64 = 1pt)
        // plus cascade damage for downstream games depending on game 0's winner
        let flipped_r64 = perfect ^ (1u64 << 0);
        let score = score_bracket(flipped_r64, perfect);
        // Game 0 wrong = -1pt. Cascade: game 0 feeds game 32, which feeds game 48, etc.
        // Lost points: 1 + 2 + 4 + 8 + 16 + 32 = 63
        assert_eq!(
            score,
            192 - 63,
            "Flipping R64 game 0 should cascade through all rounds"
        );

        // Flip bit 62 (game 62, Championship) — should lose exactly 32 points, no cascade
        let flipped_champ = perfect ^ (1u64 << 62);
        let score = score_bracket(flipped_champ, perfect);
        assert_eq!(
            score,
            192 - 32,
            "Flipping championship should lose only 32 points"
        );

        // Flip bit 32 (game 32, first R32 game) — loses 2 + cascade (4+8+16+32 = 60) = 62
        let flipped_r32 = perfect ^ (1u64 << 32);
        let score = score_bracket(flipped_r32, perfect);
        assert_eq!(
            score,
            192 - 62,
            "Flipping R32 game 32 should cascade through later rounds"
        );
    }

    #[test]
    fn test_reverse_game_bits_involution() {
        // reverse_game_bits applied twice should be identity
        let values = [
            0xFFFF_FFFF_FFFF_FFFFu64,
            0x8000_0000_0000_0000u64,
            0xBFFF_FFFF_BFFF_BFBAu64,
            0xD555_5555_5555_5555u64,
            0xC000_0000_0000_0000u64,
        ];
        for &v in &values {
            assert_eq!(
                reverse_game_bits(reverse_game_bits(v)),
                v,
                "reverse_game_bits is not an involution for 0x{:016x}",
                v
            );
        }
    }
}

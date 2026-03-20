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
/// Important: most existing off-chain Rust callers do not store brackets in the
/// bit order consumed by this exact full scorer. If your bracket bits come from
/// the current app / importer / UI pipeline, translate them first with
/// [`reverse_game_bits`] or call [`score_bracket_legacy`].
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
        // Chalky bracket — all higher seeds win
        let chalky = 0xFFFF_FFFF_FFFF_FFFEu64;
        assert_eq!(score_bracket(chalky, chalky), 192);
    }

    #[test]
    fn test_completely_wrong() {
        let all_team1 = 0xFFFF_FFFF_FFFF_FFFEu64;
        let all_team2 = 0x8000_0000_0000_0001u64;
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
    fn golden_vectors_encoding_roundtrip() {
        let vectors = load_vectors();
        let brackets = vectors["brackets"].as_array().unwrap();

        for v in brackets {
            let name = v["name"].as_str().unwrap();
            let expected_hex = v["hex"].as_str().unwrap();
            let picks = v["picks"].as_array().unwrap();

            // Encode picks to u64
            let mut bits: u64 = 1u64 << 63; // sentinel
            for (i, pick) in picks.iter().enumerate() {
                if pick.as_bool().unwrap() {
                    bits |= 1u64 << (62 - i);
                }
            }

            let actual_hex = format!("0x{:016x}", bits);
            assert_eq!(
                actual_hex, expected_hex,
                "Encoding mismatch for vector '{}'",
                name
            );

            // Verify parse roundtrip
            let parsed = parse_bracket_hex(expected_hex).unwrap();
            assert_eq!(parsed, bits, "Parse roundtrip failed for vector '{}'", name);
        }
    }

    #[test]
    fn golden_vectors_scoring() {
        let vectors = load_vectors();
        let scoring_tests = vectors["scoringTests"].as_array().unwrap();

        for st in scoring_tests {
            let description = st["description"].as_str().unwrap();
            let bracket_hex = st["bracket"].as_str().unwrap();
            let results_hex = st["results"].as_str().unwrap();
            let expected_score = st["expectedScore"].as_u64().unwrap() as u32;

            let bracket = parse_bracket_hex(bracket_hex).unwrap();
            let results = parse_bracket_hex(results_hex).unwrap();
            let actual_score = score_bracket(bracket, results);

            assert_eq!(
                actual_score, expected_score,
                "Scoring mismatch for '{}': bracket={}, results={}",
                description, bracket_hex, results_hex
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
            let bits = parse_bracket_hex(hex).unwrap();
            let score = score_bracket(bits, bits);
            assert_eq!(
                score, 192,
                "Self-score should be 192 for vector '{}' (hex={})",
                name, hex
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
                // Check sentinel: MSB must be set
                let has_sentinel = (bits >> 63) & 1 == 1;
                assert_eq!(
                    has_sentinel, expected_valid,
                    "Validation mismatch for '{}' ({}): expected valid={}",
                    hex, reason, expected_valid
                );
            } else {
                // Parse failure = invalid
                assert!(
                    !expected_valid,
                    "Parse failed for '{}' but expected valid",
                    hex
                );
            }
        }
    }
}

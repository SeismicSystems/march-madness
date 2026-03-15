//! ByteBracket scoring — Rust port of contracts/src/ByteBracket.sol.
//! Uses u64 for bit manipulation, identical logic to Solidity.

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

/// Score a bracket against results (full tournament). Max 192.
pub fn score_bracket(bracket: u64, results: u64) -> u32 {
    let filter = get_scoring_mask(results);
    score_bracket_with_mask(bracket, results, filter)
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
}

//! Legacy bracket encoding helpers for the 2026 testnet migration.
//!
//! The original UI/off-chain pipeline encoded brackets with game 0 → bit 62
//! (legacy). The contract-correct encoding is game 0 → bit 0. These helpers
//! convert between the two encodings and score legacy-encoded brackets.
//!
//! After the migration is complete, this module can be removed.

use crate::scoring::{SENTINEL_BIT, score_bracket};

/// Reverse the 63 non-sentinel game bits while preserving the sentinel bit.
///
/// Converts between legacy encoding (`game 0 → bit 62`) and contract-correct
/// encoding (`game 0 → bit 0`). The function is its own inverse (involution).
pub fn reverse_game_bits(bb: u64) -> u64 {
    let mut out = bb & SENTINEL_BIT;
    for i in 0..63u32 {
        if (bb >> i) & 1 == 1 {
            out |= 1u64 << (62 - i);
        }
    }
    out
}

/// Score a legacy-encoded bracket/results pair by first converting the 63
/// game bits into contract-correct ordering, then scoring.
pub fn score_bracket_legacy(bracket: u64, results: u64) -> u32 {
    score_bracket(reverse_game_bits(bracket), reverse_game_bits(results))
}

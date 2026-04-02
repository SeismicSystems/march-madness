//! Encode Yahoo bracket picks into the bytes8 bracket format.
//!
//! Algorithm:
//! 1. From Yahoo bracket data, extract R64 slot → (top team position, bottom team position)
//! 2. From user picks, build slot_id → winner_position for each slot
//! 3. Build (pos_lo, pos_hi) → winner_pos matchup lookup
//! 4. Simulate our bracket forward (same as decodeBracket in client/src/bracket.ts)
//! 5. Encode to u64 with sentinel bit

use std::collections::HashMap;

use eyre::{bail, eyre};
use seismic_march_madness::{encode_picks, score_bracket};
use tracing::debug;

use crate::api::{Pick, Slot};
use crate::names::NameResolver;

/// The two team positions that compete in a given Yahoo slot.
struct SlotMatchup {
    top_pos: u8,
    bottom_pos: u8,
}

/// Encode a user's Yahoo picks into a u64 bracket.
///
/// Returns (encoded_bracket, champion_name).
pub fn encode_bracket(
    slots: &[Slot],
    picks: &[Pick],
    resolver: &NameResolver,
) -> eyre::Result<(u64, String)> {
    // Step 1: Build slot_id → SlotMatchup for R64 slots (those with editorialGame)
    let mut slot_matchups: HashMap<String, SlotMatchup> = HashMap::new();
    for slot in slots {
        if let Some(ref game) = slot.editorial_game {
            let top_pos = resolver.position(&game.bracket_top_team.editorial_team_key)?;
            let bottom_pos = resolver.position(&game.bracket_bottom_team.editorial_team_key)?;
            slot_matchups.insert(
                slot.slot_id.clone(),
                SlotMatchup {
                    top_pos,
                    bottom_pos,
                },
            );
        }
    }

    // Step 2: Build slot_id → winner_position from picks, processing in round order.
    // R64 slots already have teams from editorialGame. Later rounds derive teams
    // from previousSlotIds.
    let pick_map: HashMap<String, String> = picks
        .iter()
        .map(|p| (p.slot_id.clone(), p.selected_team_key.clone()))
        .collect();

    // slot_id → winner position (filled as we process rounds)
    let mut slot_winners: HashMap<String, u8> = HashMap::new();

    // Process slots in round order (R64=1 first, up to championship=6/finalRound)
    // Sort by round_id ascending
    let mut sorted_slots: Vec<&Slot> = slots.iter().collect();
    sorted_slots.sort_by(|a, b| {
        let ra: u32 = a.round_id.parse().unwrap_or(0);
        let rb: u32 = b.round_id.parse().unwrap_or(0);
        ra.cmp(&rb)
    });

    for slot in &sorted_slots {
        let selected_key = match pick_map.get(&slot.slot_id) {
            Some(k) => k,
            None => {
                bail!("no pick for slot {}", slot.slot_id);
            }
        };

        let winner_pos = resolver.position(selected_key)?;

        if slot.editorial_game.is_some() {
            // R64: teams already known from bracket data
            slot_winners.insert(slot.slot_id.clone(), winner_pos);
        } else if slot.previous_slot_ids.len() == 2 {
            // Later round: teams come from winners of previous slots
            let prev1 = &slot.previous_slot_ids[0];
            let prev2 = &slot.previous_slot_ids[1];

            let top_pos = *slot_winners
                .get(prev1)
                .ok_or_else(|| eyre!("missing winner for previous slot {}", prev1))?;
            let bottom_pos = *slot_winners
                .get(prev2)
                .ok_or_else(|| eyre!("missing winner for previous slot {}", prev2))?;

            slot_matchups.insert(
                slot.slot_id.clone(),
                SlotMatchup {
                    top_pos,
                    bottom_pos,
                },
            );
            slot_winners.insert(slot.slot_id.clone(), winner_pos);
        } else {
            bail!(
                "slot {} has no editorialGame and {} previousSlotIds (expected 2)",
                slot.slot_id,
                slot.previous_slot_ids.len()
            );
        }
    }

    // Step 3: Build (pos_lo, pos_hi) → winner_pos matchup lookup
    let mut matchup_lookup: HashMap<(u8, u8), u8> = HashMap::new();
    for (slot_id, matchup) in &slot_matchups {
        let winner = slot_winners
            .get(slot_id)
            .ok_or_else(|| eyre!("no winner recorded for slot {}", slot_id))?;
        let lo = matchup.top_pos.min(matchup.bottom_pos);
        let hi = matchup.top_pos.max(matchup.bottom_pos);
        matchup_lookup.insert((lo, hi), *winner);
    }

    // Step 4: Simulate our bracket forward to produce 63 pick booleans.
    // Same algorithm as decodeBracket in packages/client/src/bracket.ts.
    let mut current_teams: Vec<u8> = (0..64).collect();
    let mut pick_bools: Vec<bool> = Vec::with_capacity(63);

    while current_teams.len() > 1 {
        let mut next_round = Vec::new();
        for pair in current_teams.chunks(2) {
            let a = pair[0];
            let b = pair[1];
            let (lo, hi) = if a < b { (a, b) } else { (b, a) };
            let winner = matchup_lookup
                .get(&(lo, hi))
                .ok_or_else(|| eyre!("no pick for matchup ({}, {})", lo, hi))?;
            // team1 (a) wins → true, team2 (b) wins → false
            pick_bools.push(*winner == a);
            next_round.push(*winner);
        }
        current_teams = next_round;
    }

    if pick_bools.len() != 63 {
        bail!(
            "expected 63 picks, got {} — bracket simulation error",
            pick_bools.len()
        );
    }

    // Step 5: Encode to u64
    let bits = encode_picks(&pick_bools);

    // Derive champion (last remaining team)
    let champion_pos = current_teams[0];
    let champion_name = resolver
        .ncaa_name(
            // Find which team key has this position
            picks
                .iter()
                .find_map(|p| {
                    if resolver.position(&p.selected_team_key).ok() == Some(champion_pos) {
                        Some(p.selected_team_key.as_str())
                    } else {
                        None
                    }
                })
                .unwrap_or("Unknown"),
        )
        .unwrap_or("Unknown")
        .to_string();

    // Sanity check: self-score must be 192
    let self_score = score_bracket(bits, bits);
    if self_score != 192 {
        bail!(
            "bracket self-score is {} (expected 192) — encoding error",
            self_score
        );
    }
    debug!("bracket 0x{:016x} self-scores 192 ✓", bits);

    Ok((bits, champion_name))
}

/// Format bracket bits as 0x-prefixed hex string.
pub fn format_bracket_hex(bits: u64) -> String {
    format!("0x{:016x}", bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bracket_hex() {
        assert_eq!(
            format_bracket_hex(0xFFFF_FFFF_FFFF_FFFE),
            "0xfffffffffffffffe"
        );
        assert_eq!(
            format_bracket_hex(0x8000_0000_0000_0000),
            "0x8000000000000000"
        );
    }
}

import { useMemo } from "react";
import { decodePicks } from "@march-madness/client";
import type { GameSlot } from "./useBracket";
import { getAllTeamsInBracketOrder } from "../lib/tournament";

/**
 * Compute GameSlot[] from a bracket hex string — read-only, no pick state.
 * Reuses the same game computation logic as useBracket but without interactivity.
 */
export function useReadOnlyBracket(hex: `0x${string}` | null): GameSlot[] {
  const allTeams = useMemo(() => getAllTeamsInBracketOrder(), []);

  return useMemo((): GameSlot[] => {
    if (!hex) return [];

    const picks = decodePicks(hex);

    const slots: GameSlot[] = [];
    let gameIndex = 0;

    // Round 0: R64
    for (let g = 0; g < 32; g++) {
      const team1 = allTeams[g * 2];
      const team2 = allTeams[g * 2 + 1];
      const winner = picks[gameIndex] ? team1 : team2;
      slots.push({ gameIndex, round: 0, gameInRound: g, team1, team2, winner });
      gameIndex++;
    }

    // Rounds 1-5
    let prevRoundStart = 0;
    let gamesInPrevRound = 32;

    for (let round = 1; round <= 5; round++) {
      const gamesInRound = gamesInPrevRound / 2;
      for (let g = 0; g < gamesInRound; g++) {
        const feeder1 = slots[prevRoundStart + g * 2];
        const feeder2 = slots[prevRoundStart + g * 2 + 1];
        const team1 = feeder1.winner;
        const team2 = feeder2.winner;
        let winner = null;
        if (picks[gameIndex] && team1) winner = team1;
        else if (!picks[gameIndex] && team2) winner = team2;
        slots.push({ gameIndex, round, gameInRound: g, team1, team2, winner });
        gameIndex++;
      }
      prevRoundStart += gamesInPrevRound;
      gamesInPrevRound = gamesInRound;
    }

    return slots;
  }, [allTeams, hex]);
}

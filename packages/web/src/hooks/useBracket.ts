import { useCallback, useMemo, useState } from "react";

import { encodeBracket } from "@march-madness/client";

import { getAllTeamsInBracketOrder, type Team } from "../lib/tournament";

export interface GameSlot {
  /** Index in the flat game list (0-62) */
  gameIndex: number;
  /** Round (0 = R64, 1 = R32, 2 = S16, 3 = E8, 4 = F4, 5 = Championship) */
  round: number;
  /** Game number within the round */
  gameInRound: number;
  /** The two teams that could play this game (null if not yet determined) */
  team1: Team | null;
  team2: Team | null;
  /** The picked winner for this game (null if no pick yet) */
  winner: Team | null;
}

/**
 * Hook that manages bracket pick state and encoding.
 *
 * The bracket is 63 games organized as:
 *   Round 0 (R64): games 0-31 (32 games)
 *   Round 1 (R32): games 32-47 (16 games)
 *   Round 2 (S16): games 48-55 (8 games)
 *   Round 3 (E8):  games 56-59 (4 games)
 *   Round 4 (F4):  games 60-61 (2 games)
 *   Round 5 (Championship): game 62 (1 game)
 *
 * A pick of `true` at index i means team1 (the first/higher-seeded team in the matchup) wins.
 */
export function useBracket() {
  const allTeams = useMemo(() => getAllTeamsInBracketOrder(), []);

  // picks[i] = true means team1 wins, false means team2 wins, null means no pick
  const [picks, setPicks] = useState<(boolean | null)[]>(
    new Array(63).fill(null),
  );

  /**
   * Compute all game slots based on current picks.
   * Games are numbered 0-62. The first 32 games are Round of 64 (teams from the data).
   * Later rounds derive their teams from the winners of earlier games.
   */
  const games = useMemo((): GameSlot[] => {
    const slots: GameSlot[] = [];
    let gameIndex = 0;

    // Round 0: R64 — 32 games, teams come directly from bracket order
    for (let g = 0; g < 32; g++) {
      const team1 = allTeams[g * 2];
      const team2 = allTeams[g * 2 + 1];
      const winner = picks[gameIndex] === true ? team1 : picks[gameIndex] === false ? team2 : null;
      slots.push({ gameIndex, round: 0, gameInRound: g, team1, team2, winner });
      gameIndex++;
    }

    // Rounds 1-5: derive from previous round winners
    let prevRoundStart = 0;
    let gamesInPrevRound = 32;

    for (let round = 1; round <= 5; round++) {
      const gamesInRound = gamesInPrevRound / 2;
      for (let g = 0; g < gamesInRound; g++) {
        // The two feeder games for this matchup
        const feeder1 = slots[prevRoundStart + g * 2];
        const feeder2 = slots[prevRoundStart + g * 2 + 1];
        const team1 = feeder1.winner;
        const team2 = feeder2.winner;
        const winner =
          team1 && team2 && picks[gameIndex] !== null
            ? picks[gameIndex]
              ? team1
              : team2
            : null;
        slots.push({
          gameIndex,
          round,
          gameInRound: g,
          team1,
          team2,
          winner,
        });
        gameIndex++;
      }
      prevRoundStart += gamesInPrevRound;
      gamesInPrevRound = gamesInRound;
    }

    return slots;
  }, [allTeams, picks]);

  /**
   * Make a pick for a specific game.
   * Also clears downstream picks that depended on the old winner.
   */
  const makePick = useCallback(
    (gameIndex: number, pickTeam1: boolean) => {
      setPicks((prev) => {
        const next = [...prev];
        const oldPick = next[gameIndex];

        // If same pick, do nothing
        if (oldPick === pickTeam1) return prev;

        next[gameIndex] = pickTeam1;

        // Clear downstream picks that might have used the old winner
        // We need to find which later games this game feeds into
        // and clear picks where the winner was the team that no longer advances
        clearDownstream(next, gameIndex, allTeams);

        return next;
      });
    },
    [allTeams],
  );

  /** Reset all picks */
  const resetPicks = useCallback(() => {
    setPicks(new Array(63).fill(null));
  }, []);

  /** Check if all 63 picks are made */
  const isComplete = picks.every((p) => p !== null);

  /** Encode picks to bytes8 hex (only valid if isComplete) */
  const encodedBracket = useMemo((): `0x${string}` | null => {
    if (!isComplete) return null;
    try {
      return encodeBracket(picks as boolean[]);
    } catch {
      return null;
    }
  }, [picks, isComplete]);

  /** Load picks from an existing bracket hex */
  const loadFromHex = useCallback(
    (hex: `0x${string}`) => {
      const bits = BigInt(hex);
      const newPicks: (boolean | null)[] = [];
      for (let i = 0; i < 63; i++) {
        newPicks.push(
          ((bits >> BigInt(62 - i)) & BigInt(1)) === BigInt(1),
        );
      }
      setPicks(newPicks);
    },
    [],
  );

  /** Count of picks made */
  const pickCount = picks.filter((p) => p !== null).length;

  /** Get games for a specific round */
  const getGamesForRound = useCallback(
    (round: number) => games.filter((g) => g.round === round),
    [games],
  );

  return {
    games,
    picks,
    makePick,
    resetPicks,
    isComplete,
    encodedBracket,
    loadFromHex,
    pickCount,
    allTeams,
    getGamesForRound,
  };
}

/**
 * When a pick changes, clear any downstream games where the old loser
 * was picked as a winner.
 */
function clearDownstream(
  picks: (boolean | null)[],
  changedGameIndex: number,
  allTeams: Team[],
) {
  // Recompute which team won from the changed game
  // Then walk forward through subsequent rounds
  // This is simpler: just clear all downstream games that feed from this one

  // Find what round and position this game is in
  let idx = 0;
  let round = 0;
  let roundSize = 32;
  let posInRound = 0;

  while (idx + roundSize <= changedGameIndex) {
    idx += roundSize;
    round++;
    roundSize = roundSize / 2;
  }
  posInRound = changedGameIndex - idx;

  // The next-round game that this feeds into
  let nextGamePos = Math.floor(posInRound / 2);
  let nextGameRound = round + 1;
  let nextIdx = idx + roundSize; // start of next round

  // Walk through downstream rounds and clear picks if they depend on the changed game
  while (nextGameRound <= 5) {
    const nextRoundSize = roundSize / 2;
    const gameIdx = nextIdx + nextGamePos;

    // Clear this downstream pick
    picks[gameIdx] = null;

    // Move to the next round
    nextGamePos = Math.floor(nextGamePos / 2);
    nextIdx += nextRoundSize;
    roundSize = nextRoundSize;
    nextGameRound++;
  }
}

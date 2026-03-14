import { useCallback, useEffect, useMemo, useRef, useState } from "react";

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

const ZERO_ADDR = "0x0000000000000000000000000000000000000000";
const STORAGE_PREFIX = "mm-picks-";
const storageKey = (addr: string) => `${STORAGE_PREFIX}${addr.toLowerCase()}`;

function loadPicks(addr: string): (boolean | null)[] | null {
  try {
    const raw = localStorage.getItem(storageKey(addr));
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed) && parsed.length === 63) return parsed;
  } catch {
    // corrupt data
  }
  return null;
}

function savePicks(addr: string, picks: (boolean | null)[]) {
  localStorage.setItem(storageKey(addr), JSON.stringify(picks));
}

/**
 * Hook that manages bracket pick state and encoding.
 *
 * @param walletAddress - Connected wallet address, or undefined/null if not connected.
 *   Picks are persisted in localStorage keyed by address (zero address when not connected).
 *   On login, zero-address picks migrate to the real address if the real address has no data.
 *
 * Teams can be advanced multiple rounds without their opponent decided —
 * e.g., pick Duke all the way to the championship without filling in the rest.
 */
export function useBracket(walletAddress?: string | null) {
  const allTeams = useMemo(() => getAllTeamsInBracketOrder(), []);
  const effectiveAddr = walletAddress || ZERO_ADDR;
  const prevAddrRef = useRef(effectiveAddr);

  // picks[i] = true means team1 wins, false means team2 wins, null means no pick
  const [picks, setPicks] = useState<(boolean | null)[]>(() => {
    return loadPicks(effectiveAddr) ?? new Array(63).fill(null);
  });

  // Handle address changes (login/logout) — migrate zero-address picks on login
  useEffect(() => {
    if (prevAddrRef.current === effectiveAddr) return;
    prevAddrRef.current = effectiveAddr;

    // Migrate zero-address picks to real address on login
    if (effectiveAddr !== ZERO_ADDR) {
      const zeroPicks = loadPicks(ZERO_ADDR);
      const existing = loadPicks(effectiveAddr);
      if (zeroPicks && zeroPicks.some((p) => p !== null) && !existing) {
        savePicks(effectiveAddr, zeroPicks);
      }
      localStorage.removeItem(storageKey(ZERO_ADDR));
    }

    // Load picks for the new address
    const saved = loadPicks(effectiveAddr);
    setPicks(saved ?? new Array(63).fill(null));
  }, [effectiveAddr]);

  /**
   * Compute all game slots based on current picks.
   * A team can be picked as winner even if the opponent is unknown (multi-round advancing).
   */
  const games = useMemo((): GameSlot[] => {
    const slots: GameSlot[] = [];
    let gameIndex = 0;

    // Round 0: R64 — 32 games, teams come directly from bracket order
    for (let g = 0; g < 32; g++) {
      const team1 = allTeams[g * 2];
      const team2 = allTeams[g * 2 + 1];
      const winner =
        picks[gameIndex] === true
          ? team1
          : picks[gameIndex] === false
            ? team2
            : null;
      slots.push({
        gameIndex,
        round: 0,
        gameInRound: g,
        team1,
        team2,
        winner,
      });
      gameIndex++;
    }

    // Rounds 1-5: derive from previous round winners
    let prevRoundStart = 0;
    let gamesInPrevRound = 32;

    for (let round = 1; round <= 5; round++) {
      const gamesInRound = gamesInPrevRound / 2;
      for (let g = 0; g < gamesInRound; g++) {
        const feeder1 = slots[prevRoundStart + g * 2];
        const feeder2 = slots[prevRoundStart + g * 2 + 1];
        const team1 = feeder1.winner;
        const team2 = feeder2.winner;

        // Allow winner even if opponent is unknown (multi-round advance)
        let winner: Team | null = null;
        if (picks[gameIndex] === true && team1) winner = team1;
        else if (picks[gameIndex] === false && team2) winner = team2;

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
        clearDownstream(next, gameIndex);

        savePicks(effectiveAddr, next);
        return next;
      });
    },
    [effectiveAddr],
  );

  /** Reset all picks */
  const resetPicks = useCallback(() => {
    const empty = new Array(63).fill(null) as (boolean | null)[];
    setPicks(empty);
    savePicks(effectiveAddr, empty);
  }, [effectiveAddr]);

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
      savePicks(effectiveAddr, newPicks);
    },
    [effectiveAddr],
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
 * When a pick changes, clear any downstream games whose outcome
 * may have depended on the old winner.
 */
function clearDownstream(
  picks: (boolean | null)[],
  changedGameIndex: number,
) {
  let idx = 0;
  let round = 0;
  let roundSize = 32;

  while (idx + roundSize <= changedGameIndex) {
    idx += roundSize;
    round++;
    roundSize = roundSize / 2;
  }
  const posInRound = changedGameIndex - idx;

  let nextGamePos = Math.floor(posInRound / 2);
  let nextGameRound = round + 1;
  let nextIdx = idx + roundSize;

  while (nextGameRound <= 5) {
    const nextRoundSize = roundSize / 2;
    const gameIdx = nextIdx + nextGamePos;
    picks[gameIdx] = null;
    nextGamePos = Math.floor(nextGamePos / 2);
    nextIdx += nextRoundSize;
    roundSize = nextRoundSize;
    nextGameRound++;
  }
}

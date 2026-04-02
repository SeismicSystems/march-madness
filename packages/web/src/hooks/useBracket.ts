import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import { decodePicks, encodeBracket, reverseGameBits, validateBracket } from "@march-madness/client";

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

/** Version key for one-time encoding migration. */
const ENCODING_VERSION_KEY = "mm-encoding-v";
const CURRENT_ENCODING_VERSION = "2";

/** Module-level guard so migration runs at most once per session. */
let migrationRan = false;

/**
 * One-time migration of localStorage bracket hex values from legacy encoding
 * (bit 62 = game 0) to contract-correct encoding (bit 0 = game 0).
 *
 * - Complete brackets (valid hex): apply reverseGameBits to fix bit order.
 * - Partial brackets ("partial:..." strings): no change needed — the pick
 *   string stores game-indexed picks, not bit positions.
 */
function migrateStorageEncoding(): void {
  if (migrationRan) return;
  migrationRan = true;

  try {
    if (localStorage.getItem(ENCODING_VERSION_KEY) === CURRENT_ENCODING_VERSION) return;

    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (!key || !key.startsWith(STORAGE_PREFIX)) continue;

      const value = localStorage.getItem(key);
      if (!value) continue;

      // Only migrate complete bracket hex values
      if (validateBracket(value)) {
        const migrated = reverseGameBits(value as `0x${string}`);
        localStorage.setItem(key, migrated);
      }
      // Partial brackets ("partial:...") store game-indexed picks directly — no migration needed
    }

    localStorage.setItem(ENCODING_VERSION_KEY, CURRENT_ENCODING_VERSION);
  } catch {
    // localStorage may be unavailable (private browsing, etc.)
  }
}

// Run one-time encoding migration at module load (before any loadPicks calls).
migrateStorageEncoding();

/** Sentinel prefix for incomplete (partial) brackets in localStorage. */
const PARTIAL_PREFIX = "partial:";
const createEmptyPicks = (): (boolean | null)[] => new Array(63).fill(null);

/**
 * Load picks from localStorage. Supports two formats:
 * - Complete bracket: canonical bytes8 hex string (e.g. "0x8000000000000000")
 * - Partial bracket: "partial:" + 63-char string of '1', '0', or '-' (no pick)
 */
function loadPicks(addr: string): (boolean | null)[] | null {
  try {
    const raw = localStorage.getItem(storageKey(addr));
    if (!raw) return null;

    // Complete bracket: canonical bytes8 hex
    if (validateBracket(raw)) {
      return decodePicks(raw as `0x${string}`) as (boolean | null)[];
    }

    // Partial bracket: "partial:" + 63-char pick string
    if (raw.startsWith(PARTIAL_PREFIX)) {
      const pickStr = raw.slice(PARTIAL_PREFIX.length);
      if (pickStr.length !== 63) return null;
      const picks: (boolean | null)[] = [];
      for (const ch of pickStr) {
        if (ch === "1") picks.push(true);
        else if (ch === "0") picks.push(false);
        else picks.push(null);
      }
      return picks;
    }
  } catch {
    // corrupt data
  }
  return null;
}

/**
 * Save picks to localStorage. Complete brackets are stored as canonical
 * bytes8 hex (18 chars). Incomplete brackets use a compact "partial:..."
 * format (71 chars) instead of the old JSON boolean array (~300+ chars).
 */
function savePicks(addr: string, picks: (boolean | null)[]) {
  const isComplete = picks.every((p) => p !== null);
  if (isComplete) {
    const hex = encodeBracket(picks as boolean[]);
    localStorage.setItem(storageKey(addr), hex);
  } else {
    const pickStr = picks.map((p) => (p === true ? "1" : p === false ? "0" : "-")).join("");
    localStorage.setItem(storageKey(addr), PARTIAL_PREFIX + pickStr);
  }
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
export function useBracket(
  walletAddress?: string | null,
  storageEnabled = true,
) {
  const allTeams = useMemo(() => getAllTeamsInBracketOrder(), []);
  const effectiveAddr = walletAddress || ZERO_ADDR;
  const hydratedAddrRef = useRef<string | null>(null);

  // picks[i] = true means team1 wins, false means team2 wins, null means no pick
  const [picks, setPicks] = useState<(boolean | null)[]>(createEmptyPicks);

  useEffect(() => {
    if (storageEnabled) return;
    hydratedAddrRef.current = null;
    setPicks(createEmptyPicks());
  }, [effectiveAddr, storageEnabled]);

  // Handle address changes (login/logout) once the wallet session has settled.
  useLayoutEffect(() => {
    if (!storageEnabled) return;
    if (hydratedAddrRef.current === effectiveAddr) return;
    hydratedAddrRef.current = effectiveAddr;

    // Migrate zero-address picks to real address on login
    if (effectiveAddr !== ZERO_ADDR) {
      const zeroPicks = loadPicks(ZERO_ADDR);
      const existing = loadPicks(effectiveAddr);
      if (zeroPicks && zeroPicks.some((p) => p !== null) && !existing) {
        savePicks(effectiveAddr, zeroPicks);
      }
      localStorage.removeItem(storageKey(ZERO_ADDR));
    }

    // Load picks for the new address before paint so the bracket UI
    // doesn't flash an empty state between wallet hydration steps.
    const saved = loadPicks(effectiveAddr);
    setPicks(saved ?? createEmptyPicks());
  }, [effectiveAddr, storageEnabled]);

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
    const empty = createEmptyPicks();
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
      const newPicks: (boolean | null)[] = decodePicks(hex);
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

export type UseBracketReturn = ReturnType<typeof useBracket>;

/**
 * When a pick changes, clear downstream games whose picked winner
 * came from the changed game's side of the bracket. Picks that chose
 * a team from the *other* feeder game are unaffected and preserved.
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

  let currentPos = changedGameIndex - idx;
  let nextGameRound = round + 1;
  let nextIdx = idx + roundSize;

  while (nextGameRound <= 5) {
    const nextRoundSize = roundSize / 2;
    const nextGamePos = Math.floor(currentPos / 2);
    const gameIdx = nextIdx + nextGamePos;

    // No pick here — nothing downstream depends on this path
    if (picks[gameIdx] === null) break;

    // The changed game feeds as team1 (even position) or team2 (odd position)
    const feedsAsTeam1 = currentPos % 2 === 0;
    const pickChoseChangedSide =
      (feedsAsTeam1 && picks[gameIdx] === true) ||
      (!feedsAsTeam1 && picks[gameIdx] === false);

    // If the downstream pick chose the other team, it's unaffected — stop
    if (!pickChoseChangedSide) break;

    picks[gameIdx] = null;
    currentPos = nextGamePos;
    nextIdx += nextRoundSize;
    roundSize = nextRoundSize;
    nextGameRound++;
  }
}

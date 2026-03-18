// ByteBracket scoring — TypeScript port of contracts/src/ByteBracket.sol.
// Uses BigInt for bit manipulation, identical logic to Solidity.

import type { GameStatus, PartialScore, TournamentStatus } from "./types.ts";

/** Count the number of 1-bits in a 64-bit value. */
export function popcount(bits: bigint): number {
  bits = bits & 0xFFFFFFFFFFFFFFFFn;
  bits -= (bits >> 1n) & 0x5555555555555555n;
  bits = (bits & 0x3333333333333333n) + ((bits >> 2n) & 0x3333333333333333n);
  bits = (bits + (bits >> 4n)) & 0x0F0F0F0F0F0F0F0Fn;
  return Number(((bits * 0x0101010101010101n) & 0xFFFFFFFFFFFFFFFFn) >> 56n);
}

/** Pairwise OR — takes bits two at a time and ORs them, producing half-length. */
export function pairwiseOr(bits: bigint): bigint {
  bits = bits & 0xFFFFFFFFFFFFFFFFn;
  let tmp: bigint;
  tmp = (bits ^ (bits >> 1n)) & 0x22222222n;
  bits ^= tmp ^ (tmp << 1n);
  tmp = (bits ^ (bits >> 2n)) & 0x0C0C0C0Cn;
  bits ^= tmp ^ (tmp << 2n);
  tmp = (bits ^ (bits >> 4n)) & 0x00F000F0n;
  bits ^= tmp ^ (tmp << 4n);
  tmp = (bits ^ (bits >> 8n)) & 0x0000FF00n;
  bits ^= tmp ^ (tmp << 8n);
  const evens = (bits >> 16n) & 0xFFFFFFFFn;
  const odds = bits & 0xFFFFn;
  return evens | odds;
}

/** Compute the 64-bit scoring mask from a results bracket. */
export function getScoringMask(resultsHex: `0x${string}`): bigint {
  let r = BigInt(resultsHex);
  let mask = 0n;

  // Filter for bit 62 (second MSB)
  let bitSelector = 0x4000000000000000n;
  for (let i = 0; i < 31; i++) {
    mask <<= 2n;
    if ((r & bitSelector) !== 0n) {
      mask |= 1n;
    } else {
      mask |= 2n;
    }
    r <<= 1n;
    // Keep r within 64 bits
    r &= 0xFFFFFFFFFFFFFFFFn;
  }

  return mask;
}

/**
 * Score a bracket against results (full tournament).
 * Identical to ByteBracket.getBracketScore in Solidity.
 * @returns Total points (max 192).
 */
export function scoreBracket(
  bracketHex: `0x${string}`,
  resultsHex: `0x${string}`,
): number {
  const filter = getScoringMask(resultsHex);
  return scoreBracketWithMask(bracketHex, resultsHex, filter);
}

/** Score with a precomputed mask (for batch scoring). */
export function scoreBracketWithMask(
  bracketHex: `0x${string}`,
  resultsHex: `0x${string}`,
  filter: bigint,
): number {
  let bracketBits = BigInt(bracketHex) & 0xFFFFFFFFFFFFFFFFn;
  let resultsBits = BigInt(resultsHex) & 0xFFFFFFFFFFFFFFFFn;
  let f = filter;

  let points = 0;
  let roundNum = 0;
  let numGames = 32;
  let blacklist = (1n << BigInt(numGames)) - 1n;
  let overlap = ~(bracketBits ^ resultsBits) & 0xFFFFFFFFFFFFFFFFn;

  while (numGames > 0) {
    const scores = overlap & blacklist;
    points += popcount(scores) << roundNum;
    blacklist = pairwiseOr(scores & f);
    overlap >>= BigInt(numGames);
    f >>= BigInt(numGames);
    numGames = Math.floor(numGames / 2);
    roundNum++;
  }

  return points;
}

/**
 * Score a bracket against partial tournament results (in-progress).
 *
 * `current`: points earned from decided games (simple per-game check).
 * `maxPossible`: current + best-case points from remaining undecided games,
 * accounting for elimination cascades — when a pick is wrong, the bracket's
 * predicted team is eliminated and downstream games depending on that team
 * cannot contribute to maxPossible.
 */
export function scoreBracketPartial(
  bracketHex: `0x${string}`,
  status: TournamentStatus,
): PartialScore {
  const bits = BigInt(bracketHex);

  // Extract picks: bit 62 = game 0, bit 0 = game 62.
  // pick[i] = true means bracket picks team1 (winner of feeder A) for game i.
  const picks: boolean[] = [];
  for (let i = 0; i < 63; i++) {
    picks.push(((bits >> BigInt(62 - i)) & 1n) === 1n);
  }

  // Points per round: R64=1, R32=2, S16=4, E8=8, F4=16, Champ=32
  const roundPoints = [1, 2, 4, 8, 16, 32];

  // Build game→round mapping and round start offsets
  const gameRound: number[] = [];
  const roundStart: number[] = [];
  let gamesInRound = 32;
  let offset = 0;
  for (let round = 0; round <= 5; round++) {
    roundStart.push(offset);
    for (let g = 0; g < gamesInRound; g++) {
      gameRound.push(round);
    }
    offset += gamesInRound;
    gamesInRound = Math.floor(gamesInRound / 2);
  }

  // ── current: simple per-game check (unchanged from original) ──
  let current = 0;
  for (let i = 0; i < 63; i++) {
    const game = status.games[i];
    if (game && game.status === "final" && game.winner !== undefined) {
      if (picks[i] === game.winner) {
        current += roundPoints[gameRound[i]];
      }
    }
  }

  // ── maxPossible: current + cascade-aware remaining ──
  // pickAlive[i] = the bracket's predicted team can still reach game i
  // (and the bracket's pick for game i hasn't been decided wrong).
  // For round 0: alive unless the game was decided wrong.
  // For later rounds: the feeder game supplying the bracket's picked team
  // must itself be alive, AND this game must not be decided wrong.
  const decidedWrong: boolean[] = [];
  for (let i = 0; i < 63; i++) {
    const game = status.games[i];
    if (game && game.status === "final" && game.winner !== undefined) {
      decidedWrong.push(picks[i] !== game.winner);
    } else {
      decidedWrong.push(false);
    }
  }

  const pickAlive: boolean[] = [];
  for (let i = 0; i < 63; i++) {
    const round = gameRound[i];
    if (round === 0) {
      pickAlive.push(!decidedWrong[i]);
    } else {
      const posInRound = i - roundStart[round];
      const feederA = roundStart[round - 1] + 2 * posInRound;
      const feeder = picks[i] ? feederA : feederA + 1;
      pickAlive.push(pickAlive[feeder] && !decidedWrong[i]);
    }
  }

  let maxRemaining = 0;
  for (let i = 0; i < 63; i++) {
    const game = status.games[i];
    if (!game || game.status === "upcoming" || game.status === "live") {
      if (pickAlive[i]) {
        maxRemaining += roundPoints[gameRound[i]];
      }
    }
  }

  return { current, maxPossible: current + maxRemaining };
}

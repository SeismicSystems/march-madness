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
 * For each decided game, check if the bracket pick matches.
 * For undecided games, assume the best case (all remaining picks correct)
 * to compute maxPossible.
 */
export function scoreBracketPartial(
  bracketHex: `0x${string}`,
  status: TournamentStatus,
): PartialScore {
  const bits = BigInt(bracketHex);

  // Extract picks: bit 62 = game 0, bit 0 = game 62
  const picks: boolean[] = [];
  for (let i = 0; i < 63; i++) {
    picks.push(((bits >> BigInt(62 - i)) & 1n) === 1n);
  }

  // Points per round: R64=1, R32=2, S16=4, E8=8, F4=16, Champ=32
  const roundPoints = [1, 2, 4, 8, 16, 32];

  // Build game→round mapping
  const gameRound: number[] = [];
  let gamesInRound = 32;
  for (let round = 0; round <= 5; round++) {
    for (let g = 0; g < gamesInRound; g++) {
      gameRound.push(round);
    }
    gamesInRound = Math.floor(gamesInRound / 2);
  }

  let current = 0;
  let maxPossible = 0;

  // For tracking which teams are still alive for later rounds
  // A pick can only score if the team actually reached that game
  // For simplicity in partial scoring, we score each decided game independently
  // and add full round points for undecided games as the optimistic max

  for (let i = 0; i < 63; i++) {
    const game = status.games[i];
    const round = gameRound[i];
    const pts = roundPoints[round];

    if (!game || game.status === "upcoming" || game.status === "live") {
      // Undecided — optimistically assume correct for maxPossible
      maxPossible += pts;
    } else if (game.status === "final" && game.winner !== undefined) {
      // Decided — check if bracket pick matches
      const bracketPickedTeam1 = picks[i];
      const correctPickIsTeam1 = game.winner;
      if (bracketPickedTeam1 === correctPickIsTeam1) {
        current += pts;
        maxPossible += pts;
      }
      // Wrong pick: 0 points, and downstream picks using this team also can't score.
      // For a more accurate maxPossible, we'd need to track elimination cascades.
      // This simple approach is good enough for demo — it overstates maxPossible slightly.
    }
  }

  return { current, maxPossible };
}

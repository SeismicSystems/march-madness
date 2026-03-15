import { describe, expect, test } from "bun:test";
import {
  popcount,
  pairwiseOr,
  getScoringMask,
  scoreBracket,
  scoreBracketPartial,
} from "./scoring.ts";
import type { TournamentStatus } from "./types.ts";

describe("popcount", () => {
  test("0 bits", () => {
    expect(popcount(0n)).toBe(0);
  });

  test("all 64 bits", () => {
    expect(popcount(0xFFFFFFFFFFFFFFFFn)).toBe(64);
  });

  test("single bit", () => {
    expect(popcount(1n)).toBe(1);
    expect(popcount(0x8000000000000000n)).toBe(1);
  });

  test("known value", () => {
    // 0xFF = 8 bits set
    expect(popcount(0xFFn)).toBe(8);
  });
});

describe("pairwiseOr", () => {
  test("pairs of 1s", () => {
    // 0b11 → 1, 0b10 → 1, 0b01 → 1, 0b00 → 0
    const result = pairwiseOr(0b11100100n);
    // Pairs: 11=1, 10=1, 01=1, 00=0 → 0b1110
    expect(Number(result & 0xFn)).toBe(0b1110);
  });
});

describe("scoreBracket (full)", () => {
  test("identical bracket and results scores max (192)", () => {
    // Chalky bracket — all higher seeds win = MSB sentinel + all 1s for first 63 bits
    const chalky = "0xfffffffffffffffe" as `0x${string}`;
    const score = scoreBracket(chalky, chalky);
    expect(score).toBe(192);
  });

  test("completely wrong bracket scores 0", () => {
    // Bracket: all team1 wins (bits 1-62 = 1, bit 0 = 0)
    const allTeam1 = "0xfffffffffffffffe" as `0x${string}`;
    // Results: all team2 wins (bits 1-62 = 0, bit 0 = 1) — every game bit differs
    const allTeam2 = "0x8000000000000001" as `0x${string}`;
    const score = scoreBracket(allTeam1, allTeam2);
    expect(score).toBe(0);
  });

  test("scoring is deterministic", () => {
    const bracket = "0xabcdef1234567890" as `0x${string}`;
    const results = "0xfffffffffffffffe" as `0x${string}`;
    const s1 = scoreBracket(bracket, results);
    const s2 = scoreBracket(bracket, results);
    expect(s1).toBe(s2);
  });
});

describe("scoreBracketPartial", () => {
  test("no decided games → current=0, maxPossible=192", () => {
    const status: TournamentStatus = {
      games: Array.from({ length: 63 }, (_, i) => ({
        gameIndex: i,
        status: "upcoming" as const,
      })),
    };
    const bracket = "0xfffffffffffffffe" as `0x${string}`;
    const result = scoreBracketPartial(bracket, status);
    expect(result.current).toBe(0);
    expect(result.maxPossible).toBe(192);
  });

  test("all R64 decided correctly → current=32", () => {
    // Chalky bracket: all team1 wins
    const bracket = "0xfffffffffffffffe" as `0x${string}`;
    const games = Array.from({ length: 63 }, (_, i) => {
      if (i < 32) {
        return {
          gameIndex: i,
          status: "final" as const,
          winner: true, // team1 won — matches chalky bracket
        };
      }
      return { gameIndex: i, status: "upcoming" as const };
    });
    const status: TournamentStatus = { games };
    const result = scoreBracketPartial(bracket, status);
    // 32 games × 1 point each
    expect(result.current).toBe(32);
    // 32 + remaining 31 games at their round values
    // R32: 16×2=32, S16: 8×4=32, E8: 4×8=32, F4: 2×16=32, Champ: 1×32=32
    expect(result.maxPossible).toBe(32 + 32 + 32 + 32 + 32 + 32);
  });

  test("wrong pick scores 0 for that game", () => {
    const bracket = "0xfffffffffffffffe" as `0x${string}`;
    const games = Array.from({ length: 63 }, (_, i) => {
      if (i === 0) {
        return {
          gameIndex: 0,
          status: "final" as const,
          winner: false, // team2 won — chalky bracket picked team1
        };
      }
      return { gameIndex: i, status: "upcoming" as const };
    });
    const status: TournamentStatus = { games };
    const result = scoreBracketPartial(bracket, status);
    expect(result.current).toBe(0);
  });
});

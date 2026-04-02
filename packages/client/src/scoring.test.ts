import { describe, expect, test } from "bun:test";
import { readFileSync } from "fs";
import { resolve } from "path";
import {
  popcount,
  pairwiseOr,
  getScoringMask,
  scoreBracket,
  scoreBracketPartial,
} from "./scoring.ts";
import type { TournamentStatus } from "./types.ts";

// Load golden test vectors (source of truth shared with Rust + Solidity)
const vectorsPath = resolve(__dirname, "../../../data/test-vectors/bracket-vectors.json");
const vectors = JSON.parse(readFileSync(vectorsPath, "utf-8"));

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
  // Bracket: 0xbfffffffffffffff = sentinel(1) + bit 62 = 0 + bits 61-0 all 1.
  // picks[0..61] = true (team1), picks[62] = false (team2 for championship).
  const CHALKY = "0xbfffffffffffffff" as `0x${string}`;

  // Helper: create a tournament status with all upcoming games
  function allUpcoming(): TournamentStatus {
    return {
      games: Array.from({ length: 63 }, (_, i) => ({
        gameIndex: i,
        status: "upcoming" as const,
      })),
    };
  }

  // Helper: set a game as final with a winner
  function setFinal(
    status: TournamentStatus,
    gameIndex: number,
    team1Wins: boolean,
  ): void {
    status.games[gameIndex] = {
      gameIndex,
      status: "final" as const,
      winner: team1Wins,
    };
  }

  test("no decided games → current=0, maxPossible=192", () => {
    const status = allUpcoming();
    const result = scoreBracketPartial(CHALKY, status);
    expect(result.current).toBe(0);
    expect(result.maxPossible).toBe(192);
  });

  test("all R64 decided correctly → current=32, maxPossible=192", () => {
    const status = allUpcoming();
    for (let i = 0; i < 32; i++) setFinal(status, i, true);
    const result = scoreBracketPartial(CHALKY, status);
    expect(result.current).toBe(32);
    expect(result.maxPossible).toBe(192);
  });

  test("wrong pick scores 0 for that game", () => {
    const status = allUpcoming();
    setFinal(status, 0, false); // team2 won — bracket picked team1
    const result = scoreBracketPartial(CHALKY, status);
    expect(result.current).toBe(0);
  });

  // ── Elimination cascade tests (issue #116) ──
  //
  // Key: 0xfffffffffffffffe picks team1 for games 0-61 and team2 for game 62.
  // Tournament tree feeders (game → feeder for bracket's picked team):
  //   Game 32 (R32 pos 0) picks team1 → feeder = game 0
  //   Game 48 (S16 pos 0) picks team1 → feeder = game 32
  //   Game 56 (E8 pos 0) picks team1 → feeder = game 48
  //   Game 60 (F4 pos 0) picks team1 → feeder = game 56
  //   Game 62 (Champ) picks team2  → feeder = game 61 (NOT game 60!)
  //   Game 61 (F4 pos 1) picks team1 → feeder = game 58
  //   Game 58 (E8 pos 2) picks team1 → feeder = game 52
  //   Game 52 (S16 pos 4) picks team1 → feeder = game 40
  //   Game 40 (R32 pos 8) picks team1 → feeder = game 16

  test("single wrong R64 pick cascades through downstream rounds", () => {
    // Game 0 wrong → dead chain: 32, 48, 56, 60 (all pick team1, feeder traces to game 0)
    // Game 62 picks team2 (feeder=game 61), so it's NOT in this cascade.
    // Dead undecided: 32(2) + 48(4) + 56(8) + 60(16) = 30 pts
    const status = allUpcoming();
    setFinal(status, 0, false);
    const result = scoreBracketPartial(CHALKY, status);
    expect(result.current).toBe(0);
    expect(result.maxPossible).toBe(0 + (192 - 1) - 30); // 161
  });

  test("wrong pick does NOT cascade to sibling branch", () => {
    // Game 0 wrong, games 2 and 3 correct (independent branch).
    // Game 33 (R32 pos 1, depends on games 2,3) is unaffected by game 0.
    const status = allUpcoming();
    setFinal(status, 0, false);
    setFinal(status, 2, true);
    setFinal(status, 3, true);
    const result = scoreBracketPartial(CHALKY, status);
    expect(result.current).toBe(2); // games 2 and 3
    // Dead undecided: 32(2), 48(4), 56(8), 60(16) = 30 pts
    // maxRemaining = (192 - 1 - 1 - 1) - 30 = 159
    expect(result.maxPossible).toBe(2 + 159); // 161
  });

  test("wrong pick only cascades along the bracket's predicted path", () => {
    // Flip game 32's pick to team2. Game 32 at bit position 32.
    // Now game 32 picks team2, feeder = game 1 (NOT game 0).
    // So game 0 being wrong does NOT kill game 32 or its downstream.
    const allChalk = 0xbfffffffffffffffn;
    const bit32 = 1n << 32n;
    const bracket = `0x${(allChalk ^ bit32).toString(16).padStart(16, "0")}` as `0x${string}`;

    const status = allUpcoming();
    setFinal(status, 0, false); // game 0 wrong

    const result = scoreBracketPartial(bracket, status);
    // Only game 0 itself is dead. All downstream alive (game 32 depends on game 1).
    // maxPossible = 0 + (192 - 1) = 191
    expect(result.current).toBe(0);
    expect(result.maxPossible).toBe(191);
  });

  test("two wrong R64 picks in different regions cascade independently", () => {
    // Games 0 and 16 wrong.
    // Cascade from 0: 32→48→56→60 dead
    // Cascade from 16: 40→52→58→61 dead
    // Game 62 (champ) picks team2, feeder = game 61. game 61 is dead → game 62 dead.
    const status = allUpcoming();
    setFinal(status, 0, false);
    setFinal(status, 16, false);
    const result = scoreBracketPartial(CHALKY, status);
    expect(result.current).toBe(0);
    // Dead undecided: {32,48,56,60} ∪ {40,52,58,61} ∪ {62}
    //   = 2+4+8+16 + 2+4+8+16 + 32 = 92 pts
    // maxRemaining = (192 - 1 - 1) - 92 = 98
    expect(result.maxPossible).toBe(98);
  });

  test("correct R64 + wrong R32 cascades from R32 onward", () => {
    // Games 0,1 correct. Game 32 (R32 pos 0) wrong.
    // Cascade from 32: 48→56→60 dead
    // Game 62 picks team2 (feeder=61), so NOT affected by game 60's death.
    const status = allUpcoming();
    setFinal(status, 0, true);
    setFinal(status, 1, true);
    setFinal(status, 32, false); // wrong
    const result = scoreBracketPartial(CHALKY, status);
    expect(result.current).toBe(2); // games 0 and 1
    // Dead undecided: 48(4) + 56(8) + 60(16) = 28 pts
    // maxRemaining = (192 - 1 - 1 - 2) - 28 = 160
    expect(result.maxPossible).toBe(2 + 160); // 162
  });

  test("all games decided correctly → current=maxPossible=192", () => {
    // Must set winners to match bracket picks.
    // For CHALKY: picks[0..61]=true, picks[62]=false (championship picks team2).
    const status = allUpcoming();
    for (let i = 0; i < 62; i++) setFinal(status, i, true);
    setFinal(status, 62, false); // championship: bracket picks team2
    const result = scoreBracketPartial(CHALKY, status);
    expect(result.current).toBe(192);
    expect(result.maxPossible).toBe(192);
  });

  test("coincidental bit match on dead path does NOT count toward current", () => {
    // Game 0 wrong, game 32 decided with matching bit (coincidental match).
    // The bracket predicted Team A (game 0 winner) would win game 32, but Team A
    // lost in game 0. Team B (who beat Team A) also won game 32, so the bit
    // coincidentally matches — but the bracket's team never reached game 32.
    // current must NOT count this coincidental match.
    const status = allUpcoming();
    setFinal(status, 0, false); // wrong — bracket's team eliminated
    setFinal(status, 32, true); // coincidental match — different team won via same feeder
    const result = scoreBracketPartial(CHALKY, status);
    expect(result.current).toBe(0); // no credit — bracket's team can't reach game 32
    // Dead undecided: 48(4) + 56(8) + 60(16) = 28 pts
    // maxRemaining = (192 - 1 - 2) - 28 = 161
    expect(result.maxPossible).toBe(0 + 161); // 161
  });

  test("coincidental match across multiple rounds awards zero phantom points", () => {
    // Reproduces the reported bug: bracket picks Team A through multiple rounds,
    // Team A loses in R64, but the team that beat them keeps winning — every
    // downstream game has a coincidental bit match that should NOT score.
    //
    // Game 0 (R64): bracket picks team1 (Team A), team2 (Team B) wins. WRONG.
    // Game 32 (R32): bracket picks team1 (from game 0 = Team A), Team B wins. Coincidental match.
    // Game 48 (S16): bracket picks team1 (from game 32 = Team A), Team B wins. Coincidental match.
    // Game 56 (E8): bracket picks team1 (from game 48 = Team A), Team B wins. Coincidental match.
    //
    // Without the cascade check, current would be 2+4+8=14 phantom points.
    // With the fix, current should be 0 — Team A never made it past R64.
    const status = allUpcoming();
    setFinal(status, 0, false); // R64: bracket wrong, Team A eliminated
    setFinal(status, 32, true); // R32: Team B wins (coincidental bit match)
    setFinal(status, 48, true); // S16: Team B wins (coincidental bit match)
    setFinal(status, 56, true); // E8: Team B wins (coincidental bit match)
    const result = scoreBracketPartial(CHALKY, status);
    expect(result.current).toBe(0); // zero — all matches are phantom
    // Dead undecided: 60(16) = 16 pts (game 60 still undecided but dead)
    // maxRemaining = (192 - 1 - 2 - 4 - 8) - 16 = 161
    expect(result.maxPossible).toBe(0 + 161); // 161
  });

  test("live games are treated as undecided (optimistic)", () => {
    const status = allUpcoming();
    status.games[0] = { gameIndex: 0, status: "live" as const };
    const result = scoreBracketPartial(CHALKY, status);
    expect(result.current).toBe(0);
    expect(result.maxPossible).toBe(192);
  });

  test("maxPossible is always >= current", () => {
    // Even with many wrong picks, maxPossible should never drop below current.
    const status = allUpcoming();
    for (let i = 0; i < 32; i++) setFinal(status, i, false); // all R64 wrong
    const result = scoreBracketPartial(CHALKY, status);
    expect(result.current).toBe(0);
    expect(result.maxPossible).toBeGreaterThanOrEqual(result.current);
  });

  test("all R64 wrong eliminates entire bracket", () => {
    // All 32 R64 picks wrong → every later-round pick depends on a dead feeder.
    const status = allUpcoming();
    for (let i = 0; i < 32; i++) setFinal(status, i, false);
    const result = scoreBracketPartial(CHALKY, status);
    expect(result.current).toBe(0);
    // All R32+ games depend on R64 feeders, all of which are dead.
    // maxRemaining = 0
    expect(result.maxPossible).toBe(0);
  });
});

// ── Golden vector scoring tests (cross-language consistency) ────────────

describe("golden vectors: scoring", () => {
  for (const st of vectors.scoringTests) {
    test(`${st.description}`, () => {
      const score = scoreBracket(
        st.bracket as `0x${string}`,
        st.results as `0x${string}`,
      );
      expect(score).toBe(st.expectedScore);
    });
  }

  test("self-score of every bracket is 192 (perfect)", () => {
    for (const v of vectors.brackets) {
      const score = scoreBracket(
        v.hex as `0x${string}`,
        v.hex as `0x${string}`,
      );
      expect(score).toBe(192);
    }
  });
});

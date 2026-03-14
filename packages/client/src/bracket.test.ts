import { describe, expect, test } from "bun:test";
import { encodeBracket, decodeBracket, validateBracket } from "./bracket";

describe("encodeBracket", () => {
  test("sets MSB sentinel", () => {
    const picks = new Array(63).fill(false);
    const hex = encodeBracket(picks);
    // MSB (bit 63) should be set even with all-false picks
    const val = BigInt(hex);
    expect(val >> BigInt(63)).toBe(BigInt(1));
  });

  test("rejects wrong number of picks", () => {
    expect(() => encodeBracket(new Array(62).fill(false))).toThrow(
      "Expected 63 picks",
    );
    expect(() => encodeBracket(new Array(64).fill(false))).toThrow(
      "Expected 63 picks",
    );
  });

  test("all team1 wins produces correct bits", () => {
    const picks = new Array(63).fill(true);
    const hex = encodeBracket(picks);
    // All bits set: 0xFFFFFFFFFFFFFFFF
    expect(hex).toBe("0xffffffffffffffff");
  });

  test("all team2 wins has only sentinel", () => {
    const picks = new Array(63).fill(false);
    const hex = encodeBracket(picks);
    // Only MSB set: 0x8000000000000000
    expect(hex).toBe("0x8000000000000000");
  });
});

describe("decodeBracket", () => {
  test("round-trips with encodeBracket", () => {
    const teams = Array.from({ length: 64 }, (_, i) => `Team${i}`);
    const picks = new Array(63).fill(true); // all team1 wins
    const hex = encodeBracket(picks);
    const decoded = decodeBracket(hex, teams);

    expect(decoded.champion).toBe("Team0"); // team1 always wins -> team 0 wins it all
    expect(decoded.games.length).toBe(63);
  });

  test("rejects wrong team count", () => {
    expect(() =>
      decodeBracket("0x8000000000000000", new Array(32).fill("T")),
    ).toThrow("Expected 64 teams");
  });

  test("runner-up is the loser of the championship game", () => {
    const teams = Array.from({ length: 64 }, (_, i) => `Team${i}`);
    // All team1 wins: Team0 beats everyone in their half, Team32 beats everyone in their half
    // In the championship, Team0 beats Team32
    const picks = new Array(63).fill(true);
    const hex = encodeBracket(picks);
    const decoded = decodeBracket(hex, teams);

    expect(decoded.champion).toBe("Team0");
    // Team32 is the first team in the bottom half (teams[32])
    // With all team1 wins, Team32 wins the bottom half
    expect(decoded.runnerUp).toBe("Team32");
  });
});

describe("validateBracket", () => {
  test("accepts valid brackets", () => {
    expect(validateBracket("0xffffffffffffffff")).toBe(true);
    expect(validateBracket("0x8000000000000000")).toBe(true);
    expect(validateBracket("0xABCDEF1234567890")).toBe(true);
  });

  test("rejects missing sentinel bit", () => {
    expect(validateBracket("0x7fffffffffffffff")).toBe(false);
    expect(validateBracket("0x0000000000000000")).toBe(false);
    expect(validateBracket("0x1234567890abcdef")).toBe(false);
  });

  test("rejects wrong length", () => {
    expect(validateBracket("0xffffff")).toBe(false);
    expect(validateBracket("0xffffffffffffffffff")).toBe(false);
    expect(validateBracket("")).toBe(false);
  });

  test("rejects non-hex characters", () => {
    expect(validateBracket("0xGGGGGGGGGGGGGGGG")).toBe(false);
  });

  test("rejects missing 0x prefix", () => {
    expect(validateBracket("ffffffffffffffff")).toBe(false);
  });

  test("encoded brackets always validate", () => {
    const picks1 = new Array(63).fill(true);
    const picks2 = new Array(63).fill(false);
    expect(validateBracket(encodeBracket(picks1))).toBe(true);
    expect(validateBracket(encodeBracket(picks2))).toBe(true);
  });
});

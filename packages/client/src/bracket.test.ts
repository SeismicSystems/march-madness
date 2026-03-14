import { describe, expect, test } from "bun:test";
import { encodeBracket, decodeBracket } from "./bracket";

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
});

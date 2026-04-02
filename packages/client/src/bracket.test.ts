import { describe, expect, test } from "bun:test";
import { readFileSync } from "fs";
import { resolve } from "path";
import { encodeBracket, decodeBracket, reverseGameBits, validateBracket } from "./bracket";
import { scoreBracket } from "./scoring";

// Load golden test vectors (source of truth shared with Rust + Solidity)
const vectorsPath = resolve(__dirname, "../../../data/test-vectors/bracket-vectors.json");
const vectors = JSON.parse(readFileSync(vectorsPath, "utf-8"));

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

// ── Golden vector tests (cross-language consistency) ────────────────────

describe("golden vectors: encoding", () => {
  for (const v of vectors.brackets) {
    test(`encodeBracket matches golden hex for "${v.name}"`, () => {
      const hex = encodeBracket(v.picks);
      expect(hex).toBe(v.hex);
    });
  }
});

describe("golden vectors: roundtrip", () => {
  const teams = Array.from({ length: 64 }, (_, i) => `Team${i}`);

  for (const v of vectors.brackets) {
    test(`encode/decode roundtrip for "${v.name}"`, () => {
      const hex = encodeBracket(v.picks);
      expect(hex).toBe(v.hex);

      // Decode and verify picks match
      const decoded = decodeBracket(hex, teams);
      expect(decoded.games.length).toBe(63);

      // Re-extract picks from decoded games to verify roundtrip
      const reEncoded = encodeBracket(v.picks);
      expect(reEncoded).toBe(v.hex);
    });
  }
});

// ── Bit position tests (Solidity ByteBracket layout) ──────────────────

describe("encoding: bit positions match Solidity ByteBracket layout", () => {
  test("picks[0] (first R64 game) sets bit 0", () => {
    const picks = new Array(63).fill(false);
    picks[0] = true;
    const hex = encodeBracket(picks);
    const val = BigInt(hex);
    // bit 0 should be set, plus sentinel at bit 63
    expect(val & 1n).toBe(1n);
    expect(val).toBe((1n << 63n) | 1n);
  });

  test("picks[31] (last R64 game) sets bit 31", () => {
    const picks = new Array(63).fill(false);
    picks[31] = true;
    const hex = encodeBracket(picks);
    const val = BigInt(hex);
    expect((val >> 31n) & 1n).toBe(1n);
    expect(val).toBe((1n << 63n) | (1n << 31n));
  });

  test("picks[32] (first R32 game) sets bit 32", () => {
    const picks = new Array(63).fill(false);
    picks[32] = true;
    const hex = encodeBracket(picks);
    const val = BigInt(hex);
    expect((val >> 32n) & 1n).toBe(1n);
    expect(val).toBe((1n << 63n) | (1n << 32n));
  });

  test("picks[62] (championship) sets bit 62", () => {
    const picks = new Array(63).fill(false);
    picks[62] = true;
    const hex = encodeBracket(picks);
    const val = BigInt(hex);
    expect((val >> 62n) & 1n).toBe(1n);
    expect(val).toBe((1n << 63n) | (1n << 62n));
  });

  test("round boundaries: R64=bits 0-31, R32=bits 32-47, S16=bits 48-55, E8=bits 56-59, F4=bits 60-61, Champ=bit 62", () => {
    // Set exactly one game per round and verify bit positions
    const picks = new Array(63).fill(false);
    picks[0] = true;   // R64 game 0 → bit 0
    picks[32] = true;  // R32 game 0 → bit 32
    picks[48] = true;  // S16 game 0 → bit 48
    picks[56] = true;  // E8 game 0 → bit 56
    picks[60] = true;  // F4 game 0 → bit 60
    picks[62] = true;  // Championship → bit 62

    const hex = encodeBracket(picks);
    const val = BigInt(hex);
    const expected = (1n << 63n) | (1n << 0n) | (1n << 32n) | (1n << 48n) | (1n << 56n) | (1n << 60n) | (1n << 62n);
    expect(val).toBe(expected);
  });
});

// ── reverseGameBits tests ─────────────────────────────────────────────

describe("reverseGameBits", () => {
  test("preserves sentinel bit", () => {
    const result = reverseGameBits("0x8000000000000000");
    expect(BigInt(result) >> 63n).toBe(1n);
  });

  test("sentinel-only value is unchanged", () => {
    expect(reverseGameBits("0x8000000000000000")).toBe("0x8000000000000000");
  });

  test("all-ones value is unchanged (symmetric)", () => {
    expect(reverseGameBits("0xffffffffffffffff")).toBe("0xffffffffffffffff");
  });

  test("is an involution (applying twice returns original)", () => {
    const values = [
      "0xaefefffefffffffe",
      "0xd555555555555555",
      "0xd30f00ff0000ffff",
      "0xbfffffffffffffff",
      "0xabcdef1234567890",
    ] as const;
    for (const v of values) {
      expect(reverseGameBits(reverseGameBits(v))).toBe(v);
    }
  });

  test("known value: matches Rust reverse_game_bits test", () => {
    // From Rust test: reverse_game_bits(0xC000000000000000) = 0x8000000000000001
    expect(reverseGameBits("0xc000000000000000")).toBe("0x8000000000000001");
    // And the reverse
    expect(reverseGameBits("0x8000000000000001")).toBe("0xc000000000000000");
  });

  test("bit 0 moves to bit 62 and vice versa", () => {
    // sentinel + bit 0 → sentinel + bit 62
    expect(reverseGameBits("0x8000000000000001")).toBe("0xc000000000000000");
    // sentinel + bit 62 → sentinel + bit 0
    expect(reverseGameBits("0xc000000000000000")).toBe("0x8000000000000001");
  });
});

// ── Cross-validation with Solidity contract tests ─────────────────────

describe("encoding: cross-validation with Solidity", () => {
  test("jimpo contract test: bracket=0xC000..., results=0x8000... scores 160", () => {
    // From MarchMadness.t.sol: bracket1 = 0xC000000000000000, results = 0x8000000000000000
    // Expected score = 160
    const score = scoreBracket("0xc000000000000000", "0x8000000000000000");
    expect(score).toBe(160);
  });

  test("jimpo contract test: bracket=0xF000..., results=0x8000... scores 128", () => {
    // From MarchMadness.t.sol: bracket2 = 0xF000000000000000, results = 0x8000000000000000
    // Expected score = 128
    const score = scoreBracket("0xf000000000000000", "0x8000000000000000");
    expect(score).toBe(128);
  });

  test("championship-only mismatch loses exactly 32 points (not cascaded)", () => {
    // Under correct encoding: championship = bit 62.
    // All chalk bracket with championship upset = bit 62 cleared.
    // Scoring against all-chalk results should lose only the 32-point championship round.
    const allChalk = "0xffffffffffffffff" as `0x${string}`;
    const champUpset = "0xbfffffffffffffff" as `0x${string}`;
    const score = scoreBracket(champUpset, allChalk);
    expect(score).toBe(160); // 192 - 32 = 160
  });

  test("R64 game 0 mismatch (bit 0) cascades through all downstream rounds", () => {
    // Under correct encoding: game 0 = bit 0 (LSB of R64 section).
    // When bit 0 differs, the scoring loop loses R64 match plus all downstream cascade.
    const allChalk = "0xffffffffffffffff" as `0x${string}`;
    // Flip bit 0: 0xffffffffffffffff ^ 0x1 = 0xfffffffffffffffe
    const game0Wrong = "0xfffffffffffffffe" as `0x${string}`;
    const score = scoreBracket(game0Wrong, allChalk);
    // Bit 0 wrong loses 1 (R64) + cascade: the pairwiseOr propagation kills downstream
    expect(score).toBeLessThan(192);
    expect(score).toBe(129); // loses 63 points (1+2+4+8+16+32 from cascade chain)
  });

  test("encoded bracket self-score is always 192", () => {
    // Any bracket scored against itself should get perfect score
    const testPicks = [
      new Array(63).fill(true),
      new Array(63).fill(false),
      Array.from({ length: 63 }, (_, i) => i % 2 === 0),
      Array.from({ length: 63 }, (_, i) => i < 32),
    ];
    for (const picks of testPicks) {
      const hex = encodeBracket(picks);
      expect(scoreBracket(hex, hex)).toBe(192);
    }
  });

  test("encoding roundtrip preserves scoring: encode → score matches expected", () => {
    // Encode a bracket, encode results, score them — should match contract behavior
    const bracketPicks = new Array(63).fill(true); // all chalk
    const resultsPicks = new Array(63).fill(true);
    resultsPicks[0] = false; // one upset in R64

    const bracket = encodeBracket(bracketPicks);
    const results = encodeBracket(resultsPicks);
    const score = scoreBracket(bracket, results);

    // Game 0 wrong → lose 1 (R64) + cascade downstream
    expect(score).toBe(129);
  });
});

describe("golden vectors: validation", () => {
  for (const v of vectors.validationTests) {
    test(`validateBracket("${v.hex}") = ${v.valid}: ${v.reason}`, () => {
      expect(validateBracket(v.hex)).toBe(v.valid);
    });
  }
});

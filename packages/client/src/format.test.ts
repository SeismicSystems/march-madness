import { describe, expect, test } from "bun:test";
import { encodeBracket, decodeBracket } from "./bracket";
import {
  formatBracketLines,
  formatBracketJSON,
  getFinalFourSummary,
  getTeamAdvancements,
  type TeamInfo,
} from "./format";

// Build a small set of 64 teams for testing
function makeTeams(): TeamInfo[] {
  const regions = ["East", "West", "South", "Midwest"];
  const seeds = [1, 16, 8, 9, 5, 12, 4, 13, 6, 11, 3, 14, 7, 10, 2, 15];
  const teams: TeamInfo[] = [];
  for (const region of regions) {
    for (const seed of seeds) {
      teams.push({ name: `${region}-${seed}`, seed, region });
    }
  }
  return teams;
}

describe("formatBracketLines", () => {
  test("produces one line per team", () => {
    const teams = makeTeams();
    const teamNames = teams.map((t) => t.name);
    const picks = new Array(63).fill(true); // all team1 (higher seed) wins
    const hex = encodeBracket(picks);
    const decoded = decodeBracket(hex, teamNames);
    const lines = formatBracketLines(decoded, teams);

    expect(lines.length).toBe(64);
  });

  test("champion appears first", () => {
    const teams = makeTeams();
    const teamNames = teams.map((t) => t.name);
    const picks = new Array(63).fill(true);
    const hex = encodeBracket(picks);
    const decoded = decodeBracket(hex, teamNames);
    const lines = formatBracketLines(decoded, teams);

    expect(lines[0]).toContain("Champion");
    expect(lines[0]).toContain("East-1");
  });

  test("format includes seed in parentheses", () => {
    const teams = makeTeams();
    const teamNames = teams.map((t) => t.name);
    const picks = new Array(63).fill(true);
    const hex = encodeBracket(picks);
    const decoded = decodeBracket(hex, teamNames);
    const lines = formatBracketLines(decoded, teams);

    // Champion is East-1 (seed 1)
    expect(lines[0]).toMatch(/^\(1\)/);
  });
});

describe("formatBracketJSON", () => {
  test("returns structured data for all 64 teams", () => {
    const teams = makeTeams();
    const teamNames = teams.map((t) => t.name);
    const picks = new Array(63).fill(true);
    const hex = encodeBracket(picks);
    const decoded = decodeBracket(hex, teamNames);
    const json = formatBracketJSON(decoded, teams);

    expect(json.length).toBe(64);
    expect(json[0].furthestRound).toBe("Champion");
    expect(json[0].name).toBe("East-1");
    expect(json[0].seed).toBe(1);
    expect(json[0].region).toBe("East");
  });

  test("does not include internal sort key", () => {
    const teams = makeTeams();
    const teamNames = teams.map((t) => t.name);
    const picks = new Array(63).fill(true);
    const hex = encodeBracket(picks);
    const decoded = decodeBracket(hex, teamNames);
    const json = formatBracketJSON(decoded, teams);

    // _roundIdx should be stripped
    expect("_roundIdx" in json[0]).toBe(false);
  });
});

describe("getTeamAdvancements", () => {
  test("all-team1 bracket: 1-seeds advance furthest", () => {
    const teams = makeTeams();
    const teamNames = teams.map((t) => t.name);
    const picks = new Array(63).fill(true);
    const hex = encodeBracket(picks);
    const decoded = decodeBracket(hex, teamNames);
    const advancements = getTeamAdvancements(decoded, teams);

    expect(advancements.get("East-1")).toBe("Championship");
    expect(advancements.get("West-1")).toBe("Final Four");
  });
});

describe("getFinalFourSummary", () => {
  test("returns champion and runner-up", () => {
    const teams = makeTeams();
    const teamNames = teams.map((t) => t.name);
    const picks = new Array(63).fill(true);
    const hex = encodeBracket(picks);
    const decoded = decodeBracket(hex, teamNames);
    const summary = getFinalFourSummary(decoded, teams);

    expect(summary.champion.name).toBe("East-1");
    expect(summary.runnerUp.name).toBe("South-1");
  });
});

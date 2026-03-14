// Human-readable bracket formatting.
// Converts decoded bracket data into display strings and structured JSON.

import type { DecodedBracket, BracketGame } from "./bracket.ts";

// ── Round names ─────────────────────────────────────────────────────

const ROUND_NAMES = [
  "Round of 64",
  "Round of 32",
  "Sweet 16",
  "Elite 8",
  "Final Four",
  "Championship",
] as const;

export type RoundName = (typeof ROUND_NAMES)[number];

// ── Team info for formatting ────────────────────────────────────────

export interface TeamInfo {
  name: string;
  seed: number;
  region: string;
}

/**
 * Determine how far each team advances in a bracket.
 * Returns a map from team name to their furthest round name.
 */
export function getTeamAdvancements(
  decoded: DecodedBracket,
  teams: TeamInfo[],
): Map<string, RoundName> {
  const advancements = new Map<string, RoundName>();

  // Every team at least appears in Round of 64
  for (const team of teams) {
    advancements.set(team.name, "Round of 64");
  }

  // Round of 32 winners (round index 1 in decoded.games)
  for (const game of decoded.games) {
    const roundName = ROUND_NAMES[game.round + 1]; // +1 because winning R64 means you're in R32
    if (roundName) {
      const current = advancements.get(game.winner);
      const currentIdx = current ? ROUND_NAMES.indexOf(current) : -1;
      const newIdx = ROUND_NAMES.indexOf(roundName);
      if (newIdx > currentIdx) {
        advancements.set(game.winner, roundName);
      }
    }
  }

  // The champion — winning the championship game means "Champion"
  // We override the last advancement
  advancements.set(decoded.champion, "Championship");

  return advancements;
}

/**
 * Determine the furthest round each team reaches.
 * A team "reaches" a round if they play in it (not just win into it).
 * The champion's furthest is "Champion".
 */
function getFurthestRound(
  decoded: DecodedBracket,
  teams: TeamInfo[],
): Map<string, string> {
  const furthest = new Map<string, string>();

  // All 64 teams play in Round of 64
  for (const team of teams) {
    furthest.set(team.name, "Round of 64");
  }

  // Walk through games: each winner advances to the next round
  for (const game of decoded.games) {
    const roundName = ROUND_NAMES[game.round + 1];
    if (roundName) {
      furthest.set(game.winner, roundName);
    }
  }

  // Champion label
  furthest.set(decoded.champion, "Champion");

  return furthest;
}

// ── Formatting ──────────────────────────────────────────────────────

export interface FormattedTeamLine {
  seed: number;
  name: string;
  region: string;
  furthestRound: string;
}

/**
 * Format a decoded bracket into human-readable strings.
 * Each line: "(seed) Name - furthest round"
 *
 * Teams are sorted by how far they advance (champion first), then by seed.
 *
 * @example
 * ```
 * (1) Duke - Champion
 * (2) Houston - Championship
 * (1) Michigan - Final Four
 * (3) UConn - Elite 8
 * ```
 */
export function formatBracketLines(
  decoded: DecodedBracket,
  teams: TeamInfo[],
): string[] {
  const furthest = getFurthestRound(decoded, teams);
  const teamMap = new Map(teams.map((t) => [t.name, t]));

  // Build entries with sort priority
  const entries: { line: string; roundIdx: number; seed: number }[] = [];
  for (const [name, round] of furthest) {
    const team = teamMap.get(name);
    if (!team) continue;
    const roundIdx =
      round === "Champion" ? ROUND_NAMES.length : ROUND_NAMES.indexOf(round as RoundName);
    entries.push({
      line: `(${team.seed}) ${team.name} - ${round}`,
      roundIdx,
      seed: team.seed,
    });
  }

  // Sort: furthest round descending, then seed ascending
  entries.sort((a, b) => b.roundIdx - a.roundIdx || a.seed - b.seed);

  return entries.map((e) => e.line);
}

/**
 * Format a decoded bracket into structured JSON data.
 * Returns an array of team entries with seed, name, region, and furthest round.
 * Sorted by advancement (champion first), then seed.
 */
export function formatBracketJSON(
  decoded: DecodedBracket,
  teams: TeamInfo[],
): FormattedTeamLine[] {
  const furthest = getFurthestRound(decoded, teams);
  const teamMap = new Map(teams.map((t) => [t.name, t]));

  const entries: (FormattedTeamLine & { _roundIdx: number })[] = [];
  for (const [name, round] of furthest) {
    const team = teamMap.get(name);
    if (!team) continue;
    const roundIdx =
      round === "Champion" ? ROUND_NAMES.length : ROUND_NAMES.indexOf(round as RoundName);
    entries.push({
      seed: team.seed,
      name: team.name,
      region: team.region,
      furthestRound: round,
      _roundIdx: roundIdx,
    });
  }

  entries.sort((a, b) => b._roundIdx - a._roundIdx || a.seed - b.seed);

  // Strip internal sort key
  return entries.map(({ _roundIdx, ...rest }) => rest);
}

/**
 * Get a summary of the Final Four matchup from a decoded bracket.
 * Returns the four teams that reach the Final Four, grouped by semifinal.
 */
export function getFinalFourSummary(
  decoded: DecodedBracket,
  teams: TeamInfo[],
): {
  semifinal1: [TeamInfo, TeamInfo];
  semifinal2: [TeamInfo, TeamInfo];
  champion: TeamInfo;
  runnerUp: TeamInfo;
} {
  const teamMap = new Map(teams.map((t) => [t.name, t]));

  const ff = decoded.finalFour.map((name) => teamMap.get(name)!);
  const champ = teamMap.get(decoded.champion)!;
  const runner = teamMap.get(decoded.runnerUp)!;

  return {
    semifinal1: [ff[0], ff[1]],
    semifinal2: [ff[2] ?? ff[1], ff[3] ?? ff[0]],
    champion: champ,
    runnerUp: runner,
  };
}

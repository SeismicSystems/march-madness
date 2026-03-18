import tournamentData from "@data/2026/men/tournament.json";
import { SEED_ORDER } from "./constants";

export interface FirstFourTeam {
  name: string;
  abbrev?: string;
}

export interface FirstFourEntry {
  teams: [FirstFourTeam, FirstFourTeam];
  winner?: string;
}

export interface Team {
  /** Null for First Four slots — use `displayName()` or check `firstFour`. */
  name: string | null;
  seed: number;
  region: string;
  abbrev?: string;
  firstFour?: FirstFourEntry;
}

export interface TournamentData {
  name: string;
  regions: string[];
  teams: Team[];
}

export const tournament: TournamentData = tournamentData as TournamentData;

/** Resolved display name for a team: name, FF winner, or "A/B" combo. */
export function displayName(team: Team): string {
  if (team.name) return team.name;
  if (team.firstFour) {
    if (team.firstFour.winner) return team.firstFour.winner;
    return team.firstFour.teams.map((t) => t.name).join("/");
  }
  return "TBD";
}

/** Resolved abbreviation: team abbrev, FF winner's abbrev, or combo abbrev. */
export function displayAbbrev(team: Team): string {
  if (team.name) return team.abbrev ?? team.name;
  if (team.firstFour) {
    if (team.firstFour.winner) {
      const winner = team.firstFour.teams.find(
        (t) => t.name === team.firstFour!.winner,
      );
      return winner?.abbrev ?? team.firstFour.winner;
    }
    return team.firstFour.teams.map((t) => t.abbrev ?? t.name).join("/");
  }
  return "TBD";
}

/**
 * Get teams for a region in bracket seed order.
 * The seed order is [1,16,8,9,5,12,4,13,6,11,3,14,7,10,2,15]
 * which sets up the standard bracket matchups.
 */
export function getRegionTeams(region: string): Team[] {
  const regionTeams = tournament.teams.filter((t) => t.region === region);
  return SEED_ORDER.map((seed) => regionTeams.find((t) => t.seed === seed)!);
}

/**
 * Get all 64 teams in bracket order (region by region, seed-ordered within each region).
 * Region order: East, West, South, Midwest (as defined in tournament data).
 */
export function getAllTeamsInBracketOrder(): Team[] {
  return tournament.regions.flatMap((region) => getRegionTeams(region));
}

/** Truncate an address to first 4 + last 4 chars (e.g., 0x1234...abcd) */
export function truncateAddress(address: string): string {
  if (address.length <= 10) return address;
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
}

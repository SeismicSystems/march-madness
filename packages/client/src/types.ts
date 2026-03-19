// Tournament status types — schema for live tournament data.

/** Status of a single game in the bracket (indexed 0-62). */
export interface GameStatus {
  gameIndex: number;
  status: "upcoming" | "live" | "final";
  /** Basketball score (only for live/final games). */
  score?: { team1: number; team2: number };
  /** true = team1 (higher seed) won. Only set for final games. */
  winner?: boolean;
  /** Probability that team1 wins (0-1). For live/upcoming games. */
  team1WinProbability?: number;
  /** Seconds remaining in the current period (live games only). */
  secondsRemaining?: number;
  /** Current period number (1 = 1st half, 2 = 2nd half, 3+ = OT). Live games only. */
  period?: number;
}

/** Full tournament status — served by backend, updated via POST. */
export interface TournamentStatus {
  /** 63 game statuses, indexed by gameIndex (0-62). */
  games: GameStatus[];
  /** ISO timestamp of when this status was last updated. */
  updatedAt?: string;
}

/** Entry from the indexer — matches Rust EntryRecord. */
export interface EntryRecord {
  name?: string;
  updated: { block: number; ts: number };
  bracket?: string;
}

/** The full entries index — address → EntryRecord. */
export type EntryIndex = Record<string, EntryRecord>;

/** Result of scoring a bracket against partial tournament results. */
export interface PartialScore {
  /** Points earned so far (from decided games). */
  current: number;
  /** Maximum possible points if all remaining picks are correct. */
  maxPossible: number;
}

/** Forecast for a single bracket entry (from forecaster crate). */
export interface BracketForecast {
  currentScore: number;
  maxPossibleScore: number;
  expectedScore: number;
  /** Probability of finishing with the highest score (winning the pool). 0-1. */
  winProbability: number;
  name?: string;
}

/** The full forecast index — address → BracketForecast. */
export type ForecastIndex = Record<string, BracketForecast>;

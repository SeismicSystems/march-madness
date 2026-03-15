// March Madness client library
// Abstracts over seismic-viem and the MarchMadness contract ABI.

// ── Bracket encoding/decoding/validation ────────────────────────────
export {
  encodeBracket,
  decodeBracket,
  validateBracket,
} from "./bracket.ts";
export type { BracketGame, DecodedBracket } from "./bracket.ts";

// ── Contract ABI ────────────────────────────────────────────────────
export { MarchMadnessAbi } from "./abi.ts";

// ── Client classes ──────────────────────────────────────────────────
export {
  MarchMadnessPublicClient,
  MarchMadnessUserClient,
  MarchMadnessOwnerClient,
  ENTRY_FEE,
} from "./client.ts";
export type { ReadOptions, WriteOptions } from "./client.ts";

// ── Human-readable formatting ───────────────────────────────────────
export {
  formatBracketLines,
  formatBracketJSON,
  getFinalFourSummary,
  getTeamAdvancements,
} from "./format.ts";
export type { TeamInfo, FormattedTeamLine, RoundName } from "./format.ts";

// ── Scoring ─────────────────────────────────────────────────────────
export {
  scoreBracket,
  scoreBracketWithMask,
  scoreBracketPartial,
  getScoringMask,
  popcount,
  pairwiseOr,
} from "./scoring.ts";

// ── Types ───────────────────────────────────────────────────────────
export type {
  GameStatus,
  TournamentStatus,
  EntryRecord,
  EntryIndex,
  PartialScore,
} from "./types.ts";

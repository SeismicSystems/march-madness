// March Madness client library
// Abstracts over seismic-viem and the MarchMadness, BracketGroups, and BracketMirror contracts.

// ── Bracket encoding/decoding/validation ────────────────────────────
export {
  encodeBracket,
  decodeBracket,
  validateBracket,
} from "./bracket.ts";
export type { BracketGame, DecodedBracket } from "./bracket.ts";

// ── Contract ABIs ─────────────────────────────────────────────────────
export { MarchMadnessAbi } from "./abi.ts";
export { BracketGroupsAbi } from "./abi-groups.ts";
export { BracketMirrorAbi } from "./abi-mirror.ts";

// ── MarchMadness client classes ───────────────────────────────────────
export {
  MarchMadnessPublicClient,
  MarchMadnessUserClient,
  MarchMadnessOwnerClient,
  ENTRY_FEE,
} from "./client.ts";
export type { ReadOptions, WriteOptions } from "./client.ts";

// ── BracketGroups client classes ──────────────────────────────────────
export {
  BracketGroupsPublicClient,
  BracketGroupsUserClient,
} from "./groups.ts";
export type { GroupData, MemberData, GroupPayoutData } from "./groups.ts";

// ── BracketMirror client classes ──────────────────────────────────────
export {
  BracketMirrorPublicClient,
  BracketMirrorAdminClient,
} from "./mirror.ts";
export type { MirrorData, MirrorEntryData } from "./mirror.ts";

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
  BracketForecast,
  ForecastIndex,
} from "./types.ts";

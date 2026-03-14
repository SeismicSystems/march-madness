import { ENTRY_FEE } from "@march-madness/client";
import { formatEther } from "viem";
import { sanvil } from "seismic-viem";
import deployments from "../../../../data/deployments.json";

/** Re-export ENTRY_FEE from client library */
export { ENTRY_FEE } from "@march-madness/client";

/** Entry fee display string */
export const ENTRY_FEE_DISPLAY = `${formatEther(ENTRY_FEE)} ETH`;

// ── Tournament season ────────────────────────────────────
const YEAR = "2026";
/** Unix timestamp for bracket lock: Wednesday March 18, 2026 at Noon EST */
export const SUBMISSION_DEADLINE = 1742313600;

const CHAIN_ID = import.meta.env.VITE_CHAIN_ID ?? String(sanvil.id);

/** VITE_CONTRACT_ADDRESS overrides deployments.json (populate injects this for local dev) */
export const CONTRACT_ADDRESS = (
  import.meta.env.VITE_CONTRACT_ADDRESS ??
  (deployments as Record<string, Record<string, string>>)[YEAR]?.[CHAIN_ID] ??
  "0x0000000000000000000000000000000000000000"
) as `0x${string}`;

/** Seed order per region as defined in bracket encoding */
export const SEED_ORDER = [1, 16, 8, 9, 5, 12, 4, 13, 6, 11, 3, 14, 7, 10, 2, 15];

/** Round names */
export const ROUND_NAMES = [
  "Round of 64",
  "Round of 32",
  "Sweet 16",
  "Elite 8",
  "Final Four",
  "Championship",
];

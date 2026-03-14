import { ENTRY_FEE } from "@march-madness/client";
import { formatEther } from "viem";

/** Re-export ENTRY_FEE from client library */
export { ENTRY_FEE } from "@march-madness/client";

/** Unix timestamp for bracket lock: Wednesday March 18, 2026 at Noon EST */
export const SUBMISSION_DEADLINE = 1742313600;

/** Entry fee display string */
export const ENTRY_FEE_DISPLAY = `${formatEther(ENTRY_FEE)} ETH`;

/** Contract address from env */
export const CONTRACT_ADDRESS = (import.meta.env.VITE_CONTRACT_ADDRESS ??
  "0x0000000000000000000000000000000000000000") as `0x${string}`;

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

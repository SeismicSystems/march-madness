import { ENTRY_FEE } from "@march-madness/client";
import { formatEther } from "viem";
import { sanvil, seismicTestnetGcp2 } from "seismic-viem";

/** Re-export ENTRY_FEE from client library */
export { ENTRY_FEE } from "@march-madness/client";

/** Unix timestamp for bracket lock: Wednesday March 18, 2026 at Noon EST */
export const SUBMISSION_DEADLINE = 1742313600;

/** Entry fee display string */
export const ENTRY_FEE_DISPLAY = `${formatEther(ENTRY_FEE)} ETH`;

/**
 * Known contract addresses by chain ID.
 * Testnet: hardcoded after deploy (source of truth, checked into git).
 * Sanvil: injected by populate script when it spawns vite — never set manually.
 */
const CONTRACT_ADDRESSES: Record<number, `0x${string}`> = {
  [seismicTestnetGcp2.id]: "0x0000000000000000000000000000000000000000", // TODO: update after testnet deploy
  [sanvil.id]: (import.meta.env.VITE_CONTRACT_ADDRESS ?? "0x0000000000000000000000000000000000000000") as `0x${string}`,
};

const CHAIN_ID = import.meta.env.VITE_CHAIN_ID
  ? parseInt(import.meta.env.VITE_CHAIN_ID)
  : sanvil.id;

export const CONTRACT_ADDRESS: `0x${string}` =
  CONTRACT_ADDRESSES[CHAIN_ID] ?? "0x0000000000000000000000000000000000000000";

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

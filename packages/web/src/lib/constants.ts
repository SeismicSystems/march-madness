import { ENTRY_FEE } from "@march-madness/client";
import { type Address, formatEther } from "viem";
import { sanvil } from "seismic-viem";
import deployments from "@data/deployments.json";

/** Re-export ENTRY_FEE from client library */
export { ENTRY_FEE } from "@march-madness/client";

/** Entry fee display string */
export const ENTRY_FEE_DISPLAY = `${formatEther(ENTRY_FEE)} ETH`;

// ── Tournament season ────────────────────────────────────
const YEAR = "2026";
/** Unix timestamp for bracket lock: Wednesday March 18, 2026 at Noon EST */
export const SUBMISSION_DEADLINE = 1773853200;

const CHAIN_ID = import.meta.env.VITE_CHAIN_ID ?? String(sanvil.id);

type DeploymentEntry = { marchMadness: Address; bracketGroups: Address; bracketMirror: Address };
const chainDeployment = (deployments as Record<string, Record<string, DeploymentEntry>>)[YEAR]?.[CHAIN_ID];
if (!chainDeployment) {
  throw new Error(`No deployment found for year=${YEAR} chain=${CHAIN_ID}`);
}

/** VITE_CONTRACT_ADDRESS overrides deployments.json (populate injects this for local dev) */
export const CONTRACT_ADDRESS: Address =
  (import.meta.env.VITE_CONTRACT_ADDRESS as Address | undefined) ?? chainDeployment.marchMadness;

export const GROUPS_CONTRACT_ADDRESS: Address =
  (import.meta.env.VITE_GROUPS_CONTRACT_ADDRESS as Address | undefined) ?? chainDeployment.bracketGroups;

export const MIRROR_CONTRACT_ADDRESS: Address =
  (import.meta.env.VITE_MIRROR_CONTRACT_ADDRESS as Address | undefined) ?? chainDeployment.bracketMirror;

/** Seismic testnet faucet */
export const FAUCET_URL = "https://faucet.seismictest.net";

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

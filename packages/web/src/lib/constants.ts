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
export const SUBMISSION_DEADLINE = 1773853200;

const CHAIN_ID = import.meta.env.VITE_CHAIN_ID ?? String(sanvil.id);

const ZERO_ADDRESS = "0x0000000000000000000000000000000000000000" as `0x${string}`;

// Deployment addresses: supports both old format (string) and new format (object with contract names)
type DeploymentEntry = string | { marchMadness?: string; bracketGroups?: string; bracketMirror?: string };
const chainDeployment = (deployments as Record<string, Record<string, DeploymentEntry>>)[YEAR]?.[CHAIN_ID];
const deployedAddresses = typeof chainDeployment === "string"
  ? { marchMadness: chainDeployment, bracketGroups: "", bracketMirror: "" }
  : chainDeployment ?? { marchMadness: "", bracketGroups: "", bracketMirror: "" };

/** VITE_CONTRACT_ADDRESS overrides deployments.json (populate injects this for local dev) */
export const CONTRACT_ADDRESS = (
  import.meta.env.VITE_CONTRACT_ADDRESS ?? (deployedAddresses.marchMadness || ZERO_ADDRESS)
) as `0x${string}`;

export const GROUPS_CONTRACT_ADDRESS = (
  import.meta.env.VITE_GROUPS_CONTRACT_ADDRESS ?? (deployedAddresses.bracketGroups || ZERO_ADDRESS)
) as `0x${string}`;

export const MIRROR_CONTRACT_ADDRESS = (
  import.meta.env.VITE_MIRROR_CONTRACT_ADDRESS ?? (deployedAddresses.bracketMirror || ZERO_ADDRESS)
) as `0x${string}`;

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

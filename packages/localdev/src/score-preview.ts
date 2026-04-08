/**
 * Preview a single bracket's score against candidate results on-chain.
 *
 * Calls previewScore(address, bytes8) on the MarchMadness contract — a pure
 * view function that computes what score the bracket would receive. No Redis,
 * no server, no wallet needed — just an RPC endpoint.
 *
 * Usage:
 *   bun run src/score-preview.ts --results 0x... --address 0x...
 *   bun run src/score-preview.ts --results 0x... --address 0x... --rpc-url https://...
 */

import { readFileSync } from "fs";
import { resolve } from "path";
import { http, type Address } from "viem";
import { createShieldedPublicClient, sanvil, seismicTestnetGcp2 } from "seismic-viem";
import { MarchMadnessPublicClient, scoreBracket } from "@march-madness/client";

const PROJECT_ROOT = resolve(import.meta.dir, "../../..");
const DEPLOYMENTS_PATH = resolve(PROJECT_ROOT, "data/deployments.json");
const YEAR = "2026";
const CHAIN_ID = "5124";

const SUPPORTED_CHAINS = [sanvil, seismicTestnetGcp2];

function loadEnv() {
  try {
    const content = readFileSync(resolve(PROJECT_ROOT, ".env"), "utf-8");
    for (const line of content.split("\n")) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith("#")) continue;
      const eqIdx = trimmed.indexOf("=");
      if (eqIdx === -1) continue;
      const key = trimmed.slice(0, eqIdx).trim();
      const val = trimmed.slice(eqIdx + 1).trim().replace(/^["']|["']$/g, "");
      if (!process.env[key]) process.env[key] = val;
    }
  } catch { /* no .env */ }
}

function usage(): never {
  console.error("Usage: bun run src/score-preview.ts --results 0x... --address 0x...");
  console.error("");
  console.error("Options:");
  console.error("  --results HEX    Candidate results bytes8");
  console.error("  --address ADDR   Entrant address to score");
  console.error("  --contract ADDR  Override contract address");
  console.error("  --rpc-url URL    Override RPC URL");
  console.error("  --chain-id ID    Override chain ID (default: 5124)");
  process.exit(1);
}

async function main() {
  loadEnv();

  const args = process.argv.slice(2);
  let resultsHex: string | undefined;
  let address: string | undefined;
  let contractOverride: string | undefined;
  let rpcOverride: string | undefined;
  let chainIdOverride: number | undefined;

  for (let i = 0; i < args.length; i++) {
    switch (args[i]) {
      case "--results":
        resultsHex = args[++i];
        break;
      case "--address":
        address = args[++i];
        break;
      case "--contract":
        contractOverride = args[++i];
        break;
      case "--rpc-url":
        rpcOverride = args[++i];
        break;
      case "--chain-id":
        chainIdOverride = parseInt(args[++i]);
        break;
      default:
        console.error(`Unknown option: ${args[i]}`);
        usage();
    }
  }

  if (!resultsHex || !address) usage();

  // Validate sentinel
  if ((BigInt(resultsHex) >> 63n) !== 1n) {
    console.error("ERROR: results hex missing sentinel bit");
    process.exit(1);
  }

  // Resolve contract address
  let contractAddress: Address;
  if (contractOverride) {
    contractAddress = contractOverride as Address;
  } else if (process.env.CONTRACT_ADDRESS) {
    contractAddress = process.env.CONTRACT_ADDRESS as Address;
  } else {
    const deployments = JSON.parse(readFileSync(DEPLOYMENTS_PATH, "utf-8"));
    const addr = deployments[YEAR]?.[CHAIN_ID]?.v2?.marchMadness;
    if (!addr) throw new Error(`No V2 address in deployments.json`);
    contractAddress = addr as Address;
  }

  // Resolve chain + transport
  const chainId = chainIdOverride ?? (process.env.VITE_CHAIN_ID ? parseInt(process.env.VITE_CHAIN_ID) : seismicTestnetGcp2.id);
  const chain = SUPPORTED_CHAINS.find((c) => c.id === chainId);
  if (!chain) throw new Error(`Unsupported chain ID: ${chainId}`);

  const rpcUrl = rpcOverride || process.env.VITE_RPC_URL || "http://localhost:8545";

  const publicClient = createShieldedPublicClient({ chain, transport: http(rpcUrl) });
  const mm = new MarchMadnessPublicClient(publicClient, contractAddress);

  // Read bracket from chain
  const bracket = await mm.getBracket(address as Address);
  const tag = await mm.getTag(address as Address);

  // On-chain score via previewScore
  const onChainScore = await mm.previewScore(address as Address, resultsHex as `0x${string}`);

  // Off-chain cross-check
  const offChainScore = scoreBracket(bracket, resultsHex as `0x${string}`);

  const match = onChainScore === offChainScore;

  console.log(`Address:       ${address}`);
  if (tag) console.log(`Tag:           ${tag}`);
  console.log(`Bracket:       ${bracket}`);
  console.log(`Results:       ${resultsHex}`);
  console.log(`On-chain:      ${onChainScore}/192`);
  console.log(`Off-chain:     ${offChainScore}/192`);
  console.log(`Match:         ${match ? "yes" : "MISMATCH"}`);

  if (!match) process.exit(1);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});

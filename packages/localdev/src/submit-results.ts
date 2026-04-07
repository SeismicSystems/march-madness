/**
 * Submit tournament results to the MarchMadness V2 contract.
 *
 * Workflow:
 *   1. Accept results bytes8 via --results flag (compute-results binary provides this)
 *   2. Fetch all entries from the server API
 *   3. Preview each bracket's score on-chain via previewScore(address, results)
 *   4. Display sorted leaderboard with predicted scores
 *   5. Submit results on-chain (unless --preview-only)
 *
 * Usage:
 *   bun run src/submit-results.ts --results 0x...               # preview + submit
 *   bun run src/submit-results.ts --results 0x... --preview-only # preview only
 *   bun run src/submit-results.ts --results 0x... --score-all    # submit results + score every bracket
 */

import { readFileSync } from "fs";
import { resolve } from "path";
import { createInterface } from "readline";
import { http, type Address, formatEther } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import {
  createShieldedPublicClient,
  createShieldedWalletClient,
  sanvil,
  seismicTestnetGcp2,
} from "seismic-viem";
import {
  MarchMadnessPublicClient,
  MarchMadnessOwnerClient,
  scoreBracket,
} from "@march-madness/client";
import type { EntryIndex } from "@march-madness/client";

// ── Config ───────────────────────────────────────────────────────────

const PROJECT_ROOT = resolve(import.meta.dir, "../../..");
const DEPLOYMENTS_PATH = resolve(PROJECT_ROOT, "data/deployments.json");
const YEAR = "2026";
const CHAIN_ID = "5124";

const SUPPORTED_CHAINS = [sanvil, seismicTestnetGcp2];

function getChain() {
  const chainId = process.env.VITE_CHAIN_ID
    ? parseInt(process.env.VITE_CHAIN_ID)
    : seismicTestnetGcp2.id;
  const chain = SUPPORTED_CHAINS.find((c) => c.id === chainId);
  if (!chain) throw new Error(`Unsupported VITE_CHAIN_ID: ${chainId}`);
  return chain;
}

function getTransport() {
  return http(process.env.VITE_RPC_URL || "http://localhost:8545");
}

function getContractAddress(): Address {
  if (process.env.CONTRACT_ADDRESS) {
    return process.env.CONTRACT_ADDRESS as Address;
  }
  const deployments = JSON.parse(readFileSync(DEPLOYMENTS_PATH, "utf-8"));
  const addr = deployments[YEAR]?.[CHAIN_ID]?.v2?.marchMadness;
  if (!addr) throw new Error(`No V2 marchMadness address in deployments.json for ${YEAR}/${CHAIN_ID}`);
  return addr as Address;
}

function getApiBase(): string {
  return process.env.VITE_API_BASE || "http://localhost:3000";
}

function prompt(question: string): Promise<string> {
  const rl = createInterface({ input: process.stdin, output: process.stdout });
  return new Promise((resolve) => {
    rl.question(question, (answer) => {
      rl.close();
      resolve(answer.trim());
    });
  });
}

// ── Entry fetching ───────────────────────────────────────────────────

async function fetchEntries(apiBase: string): Promise<EntryIndex> {
  const res = await fetch(`${apiBase}/entries`);
  if (!res.ok) throw new Error(`Failed to fetch entries: ${res.status} ${res.statusText}`);
  return (await res.json()) as EntryIndex;
}

// ── Main ─────────────────────────────────────────────────────────────

async function main() {
  // Load .env from repo root
  const dotenvPath = resolve(PROJECT_ROOT, ".env");
  try {
    const envContent = readFileSync(dotenvPath, "utf-8");
    for (const line of envContent.split("\n")) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith("#")) continue;
      const eqIdx = trimmed.indexOf("=");
      if (eqIdx === -1) continue;
      const key = trimmed.slice(0, eqIdx).trim();
      const val = trimmed.slice(eqIdx + 1).trim().replace(/^["']|["']$/g, "");
      if (!process.env[key]) process.env[key] = val;
    }
  } catch { /* no .env, that's fine */ }

  // Simple arg parsing (Bun-compatible, no parseArgs dependency)
  const args = process.argv.slice(2);
  const values: { results?: string; "preview-only"?: boolean; "score-all"?: boolean } = {};
  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--results" && i + 1 < args.length) {
      values.results = args[++i];
    } else if (args[i] === "--preview-only") {
      values["preview-only"] = true;
    } else if (args[i] === "--score-all") {
      values["score-all"] = true;
    }
  }

  const resultsHex = values.results as `0x${string}` | undefined;
  if (!resultsHex) {
    console.error("Usage: bun run src/submit-results.ts --results 0x...");
    console.error("");
    console.error("Compute results first:");
    console.error("  cargo run --release --bin compute-results -- --verbose");
    process.exit(1);
  }

  // Validate sentinel bit
  const resultsBigInt = BigInt(resultsHex);
  if ((resultsBigInt >> 63n) !== 1n) {
    console.error("ERROR: Results hex does not have sentinel bit set (bit 63 must be 1)");
    process.exit(1);
  }

  const contractAddress = getContractAddress();
  const chain = getChain();
  const transport = getTransport();
  const apiBase = getApiBase();

  console.log(`Contract:  ${contractAddress}`);
  console.log(`Chain:     ${chain.name} (${chain.id})`);
  console.log(`RPC:       ${process.env.VITE_RPC_URL || "http://localhost:8545"}`);
  console.log(`API:       ${apiBase}`);
  console.log(`Results:   ${resultsHex}`);
  console.log("");

  // Create public client for reads
  const publicClient = createShieldedPublicClient({
    chain,
    transport,
  });
  const mmPublic = new MarchMadnessPublicClient(publicClient, contractAddress);

  // Verify contract state
  const entryCount = await mmPublic.getEntryCount();
  const existingResults = await mmPublic.getResults();
  const entryFee = await mmPublic.getEntryFee();
  const owner = await mmPublic.getOwner();

  console.log(`Entry count:   ${entryCount}`);
  console.log(`Entry fee:     ${formatEther(entryFee)} ETH`);
  console.log(`Prize pool:    ${formatEther(entryFee * BigInt(entryCount))} ETH`);
  console.log(`Owner:         ${owner}`);

  if (existingResults !== "0x0000000000000000") {
    console.log(`\nResults already posted: ${existingResults}`);
    console.log("Cannot submit again.");
    if (!values["preview-only"]) {
      process.exit(1);
    }
  }
  console.log("");

  // ── Fetch entries ────────────────────────────────────────────────
  console.log("=== Fetching entries from server API ===");
  let entries: EntryIndex;
  try {
    entries = await fetchEntries(apiBase);
  } catch (e) {
    console.error(`Failed to fetch entries from ${apiBase}: ${e}`);
    console.error("Is the server running?");
    process.exit(1);
  }

  const addresses = Object.keys(entries).filter(
    (addr) => entries[addr].bracket,
  ) as Address[];
  console.log(`Found ${addresses.length} entries with brackets\n`);

  // ── Preview scores on-chain ──────────────────────────────────────
  console.log("=== Previewing scores via previewScore(address, bytes8) ===\n");

  type ScoreEntry = {
    address: Address;
    name: string;
    bracket: string;
    onChainScore: number;
    offChainScore: number;
    match: boolean;
  };

  const scoreEntries: ScoreEntry[] = [];

  for (const addr of addresses) {
    const entry = entries[addr];
    const bracket = entry.bracket!;
    const name = entry.name || "";

    // On-chain preview via contract
    let onChainScore: number;
    try {
      onChainScore = await mmPublic.previewScore(addr, resultsHex);
    } catch (e: any) {
      console.error(`  previewScore failed for ${addr}: ${e.message}`);
      onChainScore = -1;
    }

    // Off-chain scoring for cross-check
    const offChainScore = scoreBracket(bracket as `0x${string}`, resultsHex);

    const match = onChainScore === offChainScore;
    if (!match) {
      console.error(
        `  MISMATCH: ${addr} on-chain=${onChainScore} off-chain=${offChainScore}`,
      );
    }

    scoreEntries.push({ address: addr, name, bracket, onChainScore, offChainScore, match });
  }

  // Sort by score descending
  scoreEntries.sort((a, b) => b.onChainScore - a.onChainScore);

  // Display leaderboard
  const rankWidth = 4;
  const nameWidth = 25;
  const addrWidth = 12;

  console.log(
    `${"Rank".padEnd(rankWidth)}  ${"Name".padEnd(nameWidth)}  ${"Address".padEnd(addrWidth)}  ${"Score".padStart(5)}  ${"Off-chain".padStart(9)}  Match`,
  );
  console.log(
    `${"─".repeat(rankWidth)}  ${"─".repeat(nameWidth)}  ${"─".repeat(addrWidth)}  ${"─".repeat(5)}  ${"─".repeat(9)}  ${"─".repeat(5)}`,
  );

  let winningScore = 0;
  let numWinners = 0;

  for (let i = 0; i < scoreEntries.length; i++) {
    const e = scoreEntries[i];
    const rank = String(i + 1).padEnd(rankWidth);
    const name = (e.name || "—").slice(0, nameWidth).padEnd(nameWidth);
    const addr = `${e.address.slice(0, 6)}…${e.address.slice(-4)}`.padEnd(addrWidth);
    const score = String(e.onChainScore).padStart(5);
    const offChain = String(e.offChainScore).padStart(9);
    const matchStr = e.match ? "  ✓" : "  ✗ MISMATCH";

    console.log(`${rank}  ${name}  ${addr}  ${score}  ${offChain}  ${matchStr}`);

    if (i === 0) {
      winningScore = e.onChainScore;
      numWinners = 1;
    } else if (e.onChainScore === winningScore) {
      numWinners++;
    }
  }

  const mismatches = scoreEntries.filter((e) => !e.match);
  console.log("");
  console.log(`Winning score: ${winningScore}/192 (${numWinners} winner(s))`);
  if (numWinners > 0) {
    const payout = (entryFee * BigInt(entryCount)) / BigInt(numWinners);
    console.log(`Payout per winner: ${formatEther(payout)} ETH`);
  }

  if (mismatches.length > 0) {
    console.error(`\nWARNING: ${mismatches.length} on-chain/off-chain score mismatch(es)!`);
    console.error("Do NOT submit until mismatches are resolved.");
  }

  // ── Submit results ───────────────────────────────────────────────
  if (values["preview-only"]) {
    console.log("\nPreview only — not submitting. Run without --preview-only to submit.");
    process.exit(0);
  }

  const privateKey = process.env.DEPLOYER_PRIVATE_KEY;
  if (!privateKey) {
    console.error("\nSet DEPLOYER_PRIVATE_KEY in .env to submit results.");
    process.exit(1);
  }

  console.log("\n=== Ready to submit results ===");
  console.log(`  Contract: ${contractAddress}`);
  console.log(`  Results:  ${resultsHex}`);
  console.log(`  Winners:  ${numWinners} at score ${winningScore}`);
  console.log("");

  const answer = await prompt("Submit results on-chain? [y/N] ");
  if (answer.toLowerCase() !== "y") {
    console.log("Aborted.");
    process.exit(0);
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const walletClient: any = await createShieldedWalletClient({
    account: privateKeyToAccount(privateKey as `0x${string}`),
    chain,
    transport,
  });

  const mmOwner = new MarchMadnessOwnerClient(publicClient, walletClient, contractAddress);

  console.log("\nSubmitting results...");
  const txHash = await mmOwner.submitResults(resultsHex);
  console.log(`Transaction: ${txHash}`);

  const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });
  console.log(`Confirmed in block ${receipt.blockNumber} (status: ${receipt.status})`);

  // ── Score all brackets ───────────────────────────────────────────
  if (values["score-all"]) {
    console.log("\n=== Scoring all brackets ===");
    for (let i = 0; i < scoreEntries.length; i++) {
      const e = scoreEntries[i];
      const label = e.name || `${e.address.slice(0, 10)}...`;
      try {
        const hash = await mmOwner.scoreBracket(e.address);
        await publicClient.waitForTransactionReceipt({ hash });
        console.log(`  [${i + 1}/${scoreEntries.length}] ${label}: scored ${e.onChainScore}`);
      } catch (err: any) {
        console.error(`  [${i + 1}/${scoreEntries.length}] ${label}: FAILED — ${err.message}`);
      }
    }
    console.log("\nAll brackets scored.");
  }

  console.log("\nDone! Next steps:");
  if (!values["score-all"]) {
    console.log("  1. Score brackets (anyone can call within 7-day window):");
    console.log(`     bun run src/submit-results.ts --results ${resultsHex} --score-all`);
  }
  console.log("  2. After scoring window closes, winners call collectWinnings().");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});

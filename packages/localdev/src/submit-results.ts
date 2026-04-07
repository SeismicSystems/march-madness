/**
 * Preview and submit tournament results to the MarchMadness V2 contract.
 *
 * Default behavior is preview-only (read-only, safe to run anytime).
 * Submission requires explicit --submit flag and interactive confirmation.
 *
 * Usage:
 *   bun run submit-results -- --results 0x...                # preview: scores + groups (default)
 *   bun run submit-results -- --results 0x... --leaderboard  # just print everyone sorted by score
 *   bun run submit-results -- --results 0x... --submit       # preview + prompt to submit results
 *   bun run submit-results -- --results 0x... --score-all    # preview + prompt to submit + score all
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

// ── Types ────────────────────────────────────────────────────────────

type ScoreEntry = {
  address: Address;
  name: string;
  bracket: string;
  score: number;
};

type GroupResponse = {
  id: string;
  slug: string;
  display_name: string;
  creator: string;
  has_password: boolean;
  member_count: number;
  entry_fee: string;
};

// ── Server API fetching ─────────────────────────────────────────────

async function fetchEntries(apiBase: string): Promise<EntryIndex> {
  const res = await fetch(`${apiBase}/entries`);
  if (!res.ok) throw new Error(`Failed to fetch entries: ${res.status} ${res.statusText}`);
  return (await res.json()) as EntryIndex;
}

async function fetchGroups(apiBase: string): Promise<GroupResponse[]> {
  const res = await fetch(`${apiBase}/groups`);
  if (!res.ok) throw new Error(`Failed to fetch groups: ${res.status} ${res.statusText}`);
  return (await res.json()) as GroupResponse[];
}

async function fetchGroupMembers(apiBase: string, slug: string): Promise<string[]> {
  const res = await fetch(`${apiBase}/groups/${slug}/members`);
  if (!res.ok) throw new Error(`Failed to fetch members for group ${slug}: ${res.status}`);
  return (await res.json()) as string[];
}

// ── Display helpers ──────────────────────────────────────────────────

function printLeaderboard(entries: ScoreEntry[], opts?: { compact?: boolean }) {
  const compact = opts?.compact ?? false;
  const rankWidth = 4;
  const nameWidth = compact ? 20 : 25;
  const addrWidth = 12;

  console.log(
    `${"Rank".padEnd(rankWidth)}  ${"Name".padEnd(nameWidth)}  ${"Address".padEnd(addrWidth)}  ${"Score".padStart(5)}`,
  );
  console.log(
    `${"─".repeat(rankWidth)}  ${"─".repeat(nameWidth)}  ${"─".repeat(addrWidth)}  ${"─".repeat(5)}`,
  );

  for (let i = 0; i < entries.length; i++) {
    const e = entries[i];
    const rank = String(i + 1).padEnd(rankWidth);
    const name = (e.name || "—").slice(0, nameWidth).padEnd(nameWidth);
    const addr = `${e.address.slice(0, 6)}…${e.address.slice(-4)}`.padEnd(addrWidth);
    const score = String(e.score).padStart(5);
    console.log(`${rank}  ${name}  ${addr}  ${score}`);
  }
}

function getWinnerInfo(entries: ScoreEntry[]): { winningScore: number; numWinners: number; winners: ScoreEntry[] } {
  if (entries.length === 0) return { winningScore: 0, numWinners: 0, winners: [] };
  const winningScore = entries[0].score;
  const winners = entries.filter((e) => e.score === winningScore);
  return { winningScore, numWinners: winners.length, winners };
}

function loadEnv() {
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
}

type ParsedArgs = {
  results?: string;
  submit?: boolean;
  "score-all"?: boolean;
  leaderboard?: boolean;
};

function parseArgv(): ParsedArgs {
  const args = process.argv.slice(2);
  const values: ParsedArgs = {};
  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--results" && i + 1 < args.length) {
      values.results = args[++i];
    } else if (args[i] === "--submit") {
      values.submit = true;
    } else if (args[i] === "--score-all") {
      values["score-all"] = true;
      values.submit = true; // score-all implies submit
    } else if (args[i] === "--leaderboard") {
      values.leaderboard = true;
    }
  }
  return values;
}

// ── Score all entries from the server API ────────────────────────────

async function scoreAllEntries(
  apiBase: string,
  resultsHex: `0x${string}`,
): Promise<{ mainPool: ScoreEntry[]; scoreByAddress: Map<string, ScoreEntry> }> {
  const entries = await fetchEntries(apiBase);
  const addresses = Object.keys(entries).filter(
    (addr) => entries[addr].bracket,
  ) as Address[];

  const scoreByAddress = new Map<string, ScoreEntry>();
  for (const addr of addresses) {
    const entry = entries[addr];
    const bracket = entry.bracket!;
    const name = entry.name || "";
    const score = scoreBracket(bracket as `0x${string}`, resultsHex);
    scoreByAddress.set(addr.toLowerCase(), { address: addr, name, bracket, score });
  }

  const mainPool = [...scoreByAddress.values()].sort((a, b) => b.score - a.score);
  return { mainPool, scoreByAddress };
}

// ── Leaderboard-only mode ───────────────────────────────────────────

async function leaderboardMode(resultsHex: `0x${string}`) {
  const apiBase = getApiBase();
  console.log(`Results: ${resultsHex}`);
  console.log(`API:     ${apiBase}\n`);

  const { mainPool, scoreByAddress } = await scoreAllEntries(apiBase, resultsHex);

  console.log(`=== MAIN POOL (${mainPool.length} entries) ===\n`);
  printLeaderboard(mainPool);
  const mw = getWinnerInfo(mainPool);
  console.log(`\nWinning score: ${mw.winningScore}/192 (${mw.numWinners} winner(s))`);
  for (const w of mw.winners) {
    console.log(`  ${w.name || w.address} — ${w.score} pts`);
  }

  // Groups
  let groups: GroupResponse[] = [];
  try {
    groups = await fetchGroups(apiBase);
  } catch (e) {
    console.error(`\nFailed to fetch groups: ${e}`);
  }

  for (const group of groups) {
    let members: string[];
    try {
      members = await fetchGroupMembers(apiBase, group.slug);
    } catch {
      continue;
    }

    const groupEntries: ScoreEntry[] = [];
    for (const memberAddr of members) {
      const se = scoreByAddress.get(memberAddr.toLowerCase());
      if (se) groupEntries.push(se);
    }
    groupEntries.sort((a, b) => b.score - a.score);

    console.log(`\n=== ${group.display_name} (${group.slug}, ${members.length} members) ===\n`);
    if (groupEntries.length === 0) {
      console.log("  No scored entries");
      continue;
    }
    printLeaderboard(groupEntries, { compact: true });
    const gw = getWinnerInfo(groupEntries);
    console.log(`\nWinner: ${gw.winners.map((w) => w.name || w.address.slice(0, 10)).join(", ")} — ${gw.winningScore} pts`);
  }
}

// ── Full mode (preview + optional submit) ───────────────────────────

async function fullMode(resultsHex: `0x${string}`, values: ParsedArgs) {
  const contractAddress = getContractAddress();
  const chain = getChain();
  const transport = getTransport();
  const apiBase = getApiBase();

  console.log(`Contract:  ${contractAddress}`);
  console.log(`Chain:     ${chain.name} (${chain.id})`);
  console.log(`RPC:       ${process.env.VITE_RPC_URL || "http://localhost:8545"}`);
  console.log(`API:       ${apiBase}`);
  console.log(`Results:   ${resultsHex}\n`);

  // Create public client for reads
  const publicClient = createShieldedPublicClient({ chain, transport });
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
    if (values.submit) {
      console.log("Cannot submit again.");
      return;
    }
  }
  console.log("");

  // ── Fetch + score entries ──────────────────────────────────────────
  console.log("=== Fetching entries from server API ===");
  let mainPool: ScoreEntry[];
  let scoreByAddress: Map<string, ScoreEntry>;
  try {
    ({ mainPool, scoreByAddress } = await scoreAllEntries(apiBase, resultsHex));
  } catch (e) {
    console.error(`Failed to fetch entries from ${apiBase}: ${e}`);
    console.error("Is the server running?");
    return;
  }
  console.log(`Scored ${mainPool.length} entries (off-chain)\n`);

  // ── On-chain verification (sample) ───────────────────────────────
  const VERIFY_COUNT = Math.min(5, mainPool.length);
  if (VERIFY_COUNT > 0) {
    console.log(`Verifying top ${VERIFY_COUNT} on-chain via previewScore...`);
    let allMatch = true;
    for (let i = 0; i < VERIFY_COUNT; i++) {
      const e = mainPool[i];
      try {
        const onChainScore = await mmPublic.previewScore(e.address, resultsHex);
        const match = onChainScore === e.score;
        const label = e.name || `${e.address.slice(0, 10)}...`;
        console.log(
          `  ${label}: off-chain=${e.score} on-chain=${onChainScore} ${match ? "✓" : "✗ MISMATCH"}`,
        );
        if (!match) allMatch = false;
      } catch (err: any) {
        console.error(`  previewScore failed for ${e.address}: ${err.message}`);
        allMatch = false;
      }
    }
    if (!allMatch) {
      console.error("\nWARNING: On-chain/off-chain score mismatch detected!");
      console.error("Do NOT submit until mismatches are resolved.\n");
    } else {
      console.log("  All verified ✓\n");
    }
  }

  // ── Main pool leaderboard ────────────────────────────────────────
  console.log(`=== MAIN POOL (${mainPool.length} entries) ===\n`);
  printLeaderboard(mainPool);

  const mainWinners = getWinnerInfo(mainPool);
  console.log("");
  console.log(`Winning score: ${mainWinners.winningScore}/192 (${mainWinners.numWinners} winner(s))`);
  if (mainWinners.numWinners > 0) {
    const payout = (entryFee * BigInt(entryCount)) / BigInt(mainWinners.numWinners);
    console.log(`Payout per winner: ${formatEther(payout)} ETH`);
    for (const w of mainWinners.winners) {
      console.log(`  ${w.name || w.address} — ${w.score} pts`);
    }
  }

  // ── Group leaderboards ───────────────────────────────────────────
  let groups: GroupResponse[] = [];
  try {
    groups = await fetchGroups(apiBase);
  } catch (e) {
    console.error(`\nFailed to fetch groups: ${e}`);
  }

  if (groups.length > 0) {
    console.log(`\n${"═".repeat(60)}`);
    console.log(`=== GROUPS (${groups.length}) ===`);
    console.log(`${"═".repeat(60)}`);

    for (const group of groups) {
      let members: string[];
      try {
        members = await fetchGroupMembers(apiBase, group.slug);
      } catch {
        console.error(`\n  Could not fetch members for group "${group.display_name}"`);
        continue;
      }

      const groupEntries: ScoreEntry[] = [];
      for (const memberAddr of members) {
        const se = scoreByAddress.get(memberAddr.toLowerCase());
        if (se) groupEntries.push(se);
      }
      groupEntries.sort((a, b) => b.score - a.score);

      const groupFee = BigInt(group.entry_fee);
      const groupPrizePool = groupFee * BigInt(members.length);

      console.log(`\n--- ${group.display_name} (${group.slug}) ---`);
      console.log(`  Members: ${members.length}  |  Entry fee: ${formatEther(groupFee)} ETH  |  Prize pool: ${formatEther(groupPrizePool)} ETH`);

      if (groupEntries.length === 0) {
        console.log("  No scored entries (members may not have submitted brackets)");
        continue;
      }

      console.log("");
      printLeaderboard(groupEntries, { compact: true });

      const gw = getWinnerInfo(groupEntries);
      if (gw.numWinners > 0 && groupPrizePool > 0n) {
        const payout = groupPrizePool / BigInt(gw.numWinners);
        console.log(`  Winner: ${gw.winners.map((w) => w.name || w.address.slice(0, 10)).join(", ")} — ${gw.winningScore} pts (${formatEther(payout)} ETH each)`);
      } else {
        console.log(`  Winner: ${gw.winners.map((w) => w.name || w.address.slice(0, 10)).join(", ")} — ${gw.winningScore} pts`);
      }
    }
  }

  // ── Submit results ───────────────────────────────────────────────
  if (!values.submit) {
    console.log("\nTo submit results on-chain, re-run with --submit.");
    return;
  }

  const privateKey = process.env.DEPLOYER_PRIVATE_KEY;
  if (!privateKey) {
    console.error("\nSet DEPLOYER_PRIVATE_KEY in .env to submit results.");
    return;
  }

  console.log(`\n${"═".repeat(60)}`);
  console.log("=== SUBMIT RESULTS ===");
  console.log(`  Contract: ${contractAddress}`);
  console.log(`  Results:  ${resultsHex}`);
  console.log(`  Winners:  ${mainWinners.numWinners} at score ${mainWinners.winningScore}`);
  console.log("");
  console.log("This is irreversible. Type 'yes' to confirm.");
  console.log("");

  const answer = await prompt("> ");
  if (answer !== "yes") {
    console.log("Aborted.");
    return;
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
    for (let i = 0; i < mainPool.length; i++) {
      const e = mainPool[i];
      const label = e.name || `${e.address.slice(0, 10)}...`;
      try {
        const hash = await mmOwner.scoreBracket(e.address);
        await publicClient.waitForTransactionReceipt({ hash });
        console.log(`  [${i + 1}/${mainPool.length}] ${label}: scored ${e.score}`);
      } catch (err: any) {
        console.error(`  [${i + 1}/${mainPool.length}] ${label}: FAILED — ${err.message}`);
      }
    }
    console.log("\nAll brackets scored.");
  }

  console.log("\nDone! Next steps:");
  if (!values["score-all"]) {
    console.log("  1. Score all brackets on-chain:");
    console.log(`     bun run submit-results -- --results ${resultsHex} --score-all`);
  }
  console.log("  2. After 7-day scoring window closes, winners call collectWinnings().");
}

// ── Entry point ─────────────────────────────────────────────────────

async function main() {
  loadEnv();

  const values = parseArgv();

  const resultsHex = values.results as `0x${string}` | undefined;
  if (!resultsHex) {
    console.error("Usage: bun run submit-results -- --results 0x... [--leaderboard | --submit | --score-all]");
    console.error("");
    console.error("Options:");
    console.error("  (default)        Preview scores + groups (read-only, safe)");
    console.error("  --leaderboard    Just print everyone sorted by score (server API only, no RPC)");
    console.error("  --submit         Preview + prompt to submit results on-chain");
    console.error("  --score-all      Preview + prompt to submit + score every bracket");
    console.error("");
    console.error("Compute results first:");
    console.error("  cargo run --release --bin compute-results -- --verbose");
    throw new Error("missing --results");
  }

  // Validate sentinel bit
  if ((BigInt(resultsHex) >> 63n) !== 1n) {
    throw new Error("Results hex does not have sentinel bit set (bit 63 must be 1)");
  }

  if (values.leaderboard) {
    await leaderboardMode(resultsHex);
  } else {
    await fullMode(resultsHex, values);
  }
}

main().catch((e) => {
  console.error(e.message || e);
  process.exitCode = 1;
});

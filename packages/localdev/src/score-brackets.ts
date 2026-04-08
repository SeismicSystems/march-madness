/**
 * Score all brackets on-chain after results have been submitted.
 *
 * Reads results from the contract (already posted), scores all main pool
 * entries via scoreBracket(address), then scores all group entries via
 * scoreEntry(groupId, memberIndex). Skips already-scored entries.
 *
 * Usage:
 *   bun run score-brackets                        # score all (main pool + groups)
 *   bun run score-brackets -- --dry-run            # preview who needs scoring, don't send txs
 *   bun run score-brackets -- --main-only          # skip group scoring
 *   bun run score-brackets -- --groups-only        # skip main pool scoring
 */

import { readFileSync } from "fs";
import { resolve } from "path";
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
  MarchMadnessUserClient,
  BracketGroupsPublicClient,
  BracketGroupsUserClient,
  scoreBracket,
} from "@march-madness/client";
import type { EntryIndex } from "@march-madness/client";

// ── Config ───────────────────────────────────────────────────────────

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
      const val = trimmed
        .slice(eqIdx + 1)
        .trim()
        .replace(/^["']|["']$/g, "");
      if (!process.env[key]) process.env[key] = val;
    }
  } catch {
    /* no .env */
  }
}

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

function getDeployments() {
  const deployments = JSON.parse(readFileSync(DEPLOYMENTS_PATH, "utf-8"));
  const v2 = deployments[YEAR]?.[CHAIN_ID]?.v2;
  if (!v2?.marchMadness)
    throw new Error(
      `No V2 marchMadness address in deployments.json for ${YEAR}/${CHAIN_ID}`,
    );
  if (!v2?.bracketGroups)
    throw new Error(
      `No V2 bracketGroups address in deployments.json for ${YEAR}/${CHAIN_ID}`,
    );
  return {
    marchMadness: (process.env.CONTRACT_ADDRESS || v2.marchMadness) as Address,
    bracketGroups: (process.env.GROUPS_CONTRACT_ADDRESS ||
      v2.bracketGroups) as Address,
  };
}

function getApiBase(): string {
  return process.env.VITE_API_BASE || "http://localhost:3000";
}

// ── Types ────────────────────────────────────────────────────────────

type ParsedArgs = {
  dryRun: boolean;
  mainOnly: boolean;
  groupsOnly: boolean;
};

function parseArgv(): ParsedArgs {
  const args = process.argv.slice(2);
  const values: ParsedArgs = {
    dryRun: false,
    mainOnly: false,
    groupsOnly: false,
  };
  for (const arg of args) {
    if (arg === "--dry-run") values.dryRun = true;
    else if (arg === "--main-only") values.mainOnly = true;
    else if (arg === "--groups-only") values.groupsOnly = true;
    else {
      console.error(`Unknown option: ${arg}`);
      process.exit(1);
    }
  }
  return values;
}

type GroupResponse = {
  id: string;
  slug: string;
  display_name: string;
  member_count: number;
  entry_fee: string;
};

// ── Server API ──────────────────────────────────────────────────────

async function fetchEntries(apiBase: string): Promise<EntryIndex> {
  const res = await fetch(`${apiBase}/entries`);
  if (!res.ok)
    throw new Error(
      `Failed to fetch entries: ${res.status} ${res.statusText}`,
    );
  return (await res.json()) as EntryIndex;
}

async function fetchGroups(apiBase: string): Promise<GroupResponse[]> {
  const res = await fetch(`${apiBase}/groups`);
  if (!res.ok)
    throw new Error(
      `Failed to fetch groups: ${res.status} ${res.statusText}`,
    );
  return (await res.json()) as GroupResponse[];
}

// ── Main ────────────────────────────────────────────────────────────

async function main() {
  loadEnv();
  const args = parseArgv();

  const { marchMadness: mmAddr, bracketGroups: groupsAddr } = getDeployments();
  const chain = getChain();
  const transport = getTransport();
  const apiBase = getApiBase();

  console.log(`MarchMadness:   ${mmAddr}`);
  console.log(`BracketGroups:  ${groupsAddr}`);
  console.log(`Chain:          ${chain.name} (${chain.id})`);
  console.log(`API:            ${apiBase}`);
  if (args.dryRun) console.log(`Mode:           DRY RUN (no transactions)`);
  console.log("");

  // ── Create clients ────────────────────────────────────────────────

  const publicClient = createShieldedPublicClient({ chain, transport });
  const mmPublic = new MarchMadnessPublicClient(publicClient, mmAddr);

  // Check results are posted
  const results = await mmPublic.getResults();
  if (results === "0x0000000000000000") {
    console.error("ERROR: Results have not been posted yet. Nothing to score.");
    process.exit(1);
  }
  console.log(`Results:        ${results}`);

  // We need a wallet client for scoring transactions
  let mmUser: MarchMadnessUserClient | null = null;
  let groupsUser: BracketGroupsUserClient | null = null;

  if (!args.dryRun) {
    const privateKey = process.env.DEPLOYER_PRIVATE_KEY;
    if (!privateKey) {
      console.error(
        "Set DEPLOYER_PRIVATE_KEY in .env to score brackets on-chain.",
      );
      process.exit(1);
    }

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const walletClient: any = await createShieldedWalletClient({
      account: privateKeyToAccount(privateKey as `0x${string}`),
      chain,
      transport,
    });

    mmUser = new MarchMadnessUserClient(
      publicClient,
      walletClient,
      mmAddr,
    );
    groupsUser = new BracketGroupsUserClient(
      publicClient,
      walletClient,
      groupsAddr,
    );

    console.log(`Signer:         ${walletClient.account.address}`);
  }

  const groupsPublic = new BracketGroupsPublicClient(publicClient, groupsAddr);
  console.log("");

  // ── Score main pool ───────────────────────────────────────────────

  if (!args.groupsOnly) {
    console.log("=== MAIN POOL ===\n");

    const entries = await fetchEntries(apiBase);
    const addresses = Object.keys(entries).filter(
      (addr) => entries[addr].bracket,
    ) as Address[];

    console.log(`Total entries with brackets: ${addresses.length}`);

    // Check which are already scored
    const needsScoring: { address: Address; name: string; offChainScore: number }[] = [];
    const alreadyScored: { address: Address; name: string }[] = [];

    for (const addr of addresses) {
      const isScored = await mmPublic.getIsScored(addr);
      const entry = entries[addr];
      const name = entry.name || "";
      if (isScored) {
        alreadyScored.push({ address: addr, name });
      } else {
        const offChainScore = scoreBracket(
          entry.bracket! as `0x${string}`,
          results as `0x${string}`,
        );
        needsScoring.push({ address: addr, name, offChainScore });
      }
    }

    // Sort by score descending for nice output
    needsScoring.sort((a, b) => b.offChainScore - a.offChainScore);

    console.log(`Already scored: ${alreadyScored.length}`);
    console.log(`Needs scoring:  ${needsScoring.length}\n`);

    if (needsScoring.length > 0 && !args.dryRun) {
      for (let i = 0; i < needsScoring.length; i++) {
        const e = needsScoring[i];
        const label = e.name || `${e.address.slice(0, 8)}…`;
        try {
          const hash = await mmUser!.scoreBracket(e.address);
          await publicClient.waitForTransactionReceipt({ hash });
          console.log(
            `  [${i + 1}/${needsScoring.length}] ${label}: scored ${e.offChainScore} pts`,
          );
        } catch (err: any) {
          console.error(
            `  [${i + 1}/${needsScoring.length}] ${label}: FAILED — ${err.message}`,
          );
        }
      }

      // Print final state
      const winningScore = await mmPublic.getWinningScore();
      const numWinners = await mmPublic.getNumWinners();
      console.log(
        `\nMain pool: winningScore=${winningScore}, numWinners=${numWinners}`,
      );
    } else if (needsScoring.length > 0) {
      console.log("Entries that need scoring:");
      for (const e of needsScoring) {
        const label = e.name || e.address;
        console.log(`  ${label} — ${e.offChainScore} pts (predicted)`);
      }
    } else {
      console.log("All main pool entries already scored.");
    }
  }

  // ── Score groups ──────────────────────────────────────────────────

  if (!args.mainOnly) {
    console.log("\n=== GROUPS ===\n");

    let groups: GroupResponse[];
    try {
      groups = await fetchGroups(apiBase);
    } catch (e) {
      console.error(`Failed to fetch groups: ${e}`);
      return;
    }

    console.log(`Found ${groups.length} group(s)\n`);

    for (const group of groups) {
      // Resolve groupId from the contract via slug
      let groupId: number;
      try {
        const [id] = await groupsPublic.getGroupBySlug(group.slug);
        groupId = id;
      } catch (err: any) {
        console.error(
          `  Could not resolve group "${group.slug}": ${err.message}`,
        );
        continue;
      }

      // Get members from the contract (need indices for scoreEntry)
      const members = await groupsPublic.getMembers(groupId);

      const needsGroupScoring: { index: number; addr: Address; name: string }[] = [];
      const alreadyGroupScored: { addr: Address; name: string }[] = [];

      for (let idx = 0; idx < members.length; idx++) {
        const m = members[idx];
        if (m.isScored) {
          alreadyGroupScored.push({ addr: m.addr, name: m.name });
        } else {
          needsGroupScoring.push({ index: idx, addr: m.addr, name: m.name });
        }
      }

      console.log(
        `--- ${group.display_name} (${group.slug}, id=${groupId}) ---`,
      );
      console.log(
        `  Members: ${members.length}  |  Already scored: ${alreadyGroupScored.length}  |  Needs scoring: ${needsGroupScoring.length}`,
      );

      if (needsGroupScoring.length > 0 && !args.dryRun) {
        for (let i = 0; i < needsGroupScoring.length; i++) {
          const m = needsGroupScoring[i];
          const label = m.name || `${m.addr.slice(0, 8)}…`;
          try {
            const hash = await groupsUser!.scoreEntry(groupId, m.index);
            await publicClient.waitForTransactionReceipt({ hash });
            console.log(
              `  [${i + 1}/${needsGroupScoring.length}] ${label}: scored`,
            );
          } catch (err: any) {
            console.error(
              `  [${i + 1}/${needsGroupScoring.length}] ${label}: FAILED — ${err.message}`,
            );
          }
        }

        // Print group payout state
        const payout = await groupsPublic.getPayouts(groupId);
        console.log(
          `  Group result: winningScore=${payout.winningScore}, numWinners=${payout.numWinners}, numScored=${payout.numScored}`,
        );
      } else if (needsGroupScoring.length > 0) {
        for (const m of needsGroupScoring) {
          console.log(`  Needs scoring: ${m.name || m.addr} (index ${m.index})`);
        }
      } else {
        console.log(`  All members already scored.`);
      }
      console.log("");
    }
  }

  console.log("Done.");
}

main().catch((e) => {
  console.error(e.message || e);
  process.exitCode = 1;
});

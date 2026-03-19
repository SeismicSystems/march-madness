/**
 * Yahoo Mirror — reads platform.json from the Rust mirror-importer,
 * creates/updates BracketMirror entries on-chain.
 *
 * Usage: bun run src/yahoo-mirror.ts -- --group-id 21309
 */

import { readFileSync, writeFileSync, existsSync } from "fs";
import { resolve } from "path";
import { http } from "viem";
import type { Address, Hex } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import {
  createShieldedWalletClient,
  createShieldedPublicClient,
  sanvil,
  seismicTestnetGcp2,
} from "seismic-viem";
import { BracketMirrorAdminClient } from "@march-madness/client";

// ── Paths ─────────────────────────────────────────────────────────────

const PROJECT_ROOT = resolve(import.meta.dir, "../../..");
const DATA_DIR = resolve(PROJECT_ROOT, "data");

// Load root .env (bun only auto-loads .env from cwd, not repo root)
const rootEnv = resolve(PROJECT_ROOT, ".env");
if (existsSync(rootEnv)) {
  for (const line of readFileSync(rootEnv, "utf-8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eq = trimmed.indexOf("=");
    if (eq === -1) continue;
    const key = trimmed.slice(0, eq);
    let val = trimmed.slice(eq + 1);
    // Strip surrounding quotes
    if ((val.startsWith("'") && val.endsWith("'")) || (val.startsWith('"') && val.endsWith('"'))) {
      val = val.slice(1, -1);
    }
    if (!process.env[key]) process.env[key] = val;
  }
}

// ── Types ─────────────────────────────────────────────────────────────

interface PlatformEntry {
  team_id: string;
  name: string;
  user: string;
  bracket: string;
  champion: string;
}

interface PlatformOutput {
  slug: string;
  group_id: number;
  year: number;
  entries: PlatformEntry[];
}

interface OnChainMirror {
  slug: string;
  id: number;
}

interface OnChainMember {
  mirrorEntryId: number;
  slug: string;
}

interface OnChainOutput {
  mirror: OnChainMirror;
  members: Record<string, OnChainMember>;
}

// ── Args ──────────────────────────────────────────────────────────────

function parseArgs(): { groupId: number } {
  const args = process.argv.slice(2);
  let groupId: number | null = null;

  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--group-id" && i + 1 < args.length) {
      groupId = parseInt(args[i + 1], 10);
      i++;
    }
  }

  if (groupId === null || isNaN(groupId)) {
    console.error("Usage: bun run src/yahoo-mirror.ts -- --group-id <id>");
    process.exit(1);
  }

  return { groupId };
}

// ── Deployment loading ────────────────────────────────────────────────

function loadMirrorAddress(): Address {
  const deploymentsPath = resolve(DATA_DIR, "deployments.json");
  const deployments = JSON.parse(readFileSync(deploymentsPath, "utf-8"));

  // Find the mirror address — try 2026 first, then any year
  for (const year of Object.keys(deployments)) {
    for (const chainId of Object.keys(deployments[year])) {
      const addrs = deployments[year][chainId];
      if (addrs.bracketMirror) {
        return addrs.bracketMirror as Address;
      }
    }
  }

  throw new Error("No bracketMirror address found in data/deployments.json");
}

// ── Chain / client setup (mirrors webapp pattern) ────────────────────

const SUPPORTED_CHAINS = [sanvil, seismicTestnetGcp2];

function getChain() {
  const chainId = process.env.VITE_CHAIN_ID
    ? parseInt(process.env.VITE_CHAIN_ID)
    : sanvil.id;
  const chain = SUPPORTED_CHAINS.find((c) => c.id === chainId);
  if (!chain) throw new Error(`Unsupported VITE_CHAIN_ID: ${chainId}`);
  return chain;
}

function getTransport() {
  return http(process.env.VITE_RPC_URL || "http://localhost:8545");
}

// ── Slug generation ───────────────────────────────────────────────────

function makeEntrySlug(entry: PlatformEntry): string {
  // Use bracket name, sanitized for on-chain slug
  return entry.name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 50);
}

// ── Main ──────────────────────────────────────────────────────────────

async function main() {
  const { groupId } = parseArgs();

  // Load platform.json
  const cacheDir = resolve(
    DATA_DIR,
    "cache",
    "mirrors",
    "yahoo",
    "groups",
    String(groupId),
  );
  const platformPath = resolve(cacheDir, "platform.json");
  if (!existsSync(platformPath)) {
    console.error(
      `platform.json not found at ${platformPath}\nRun the Rust mirror-importer first: cargo run -p mirror-importer -- --group-id ${groupId}`,
    );
    process.exit(1);
  }

  const platform: PlatformOutput = JSON.parse(
    readFileSync(platformPath, "utf-8"),
  );
  console.log(
    `loaded platform.json: ${platform.entries.length} entries, slug="${platform.slug}"`,
  );

  // Load deployer private key
  const deployerKey = process.env.DEPLOYER_PRIVATE_KEY;
  if (!deployerKey) {
    throw new Error("DEPLOYER_PRIVATE_KEY env var required");
  }

  // Create clients
  const chain = getChain();
  const transport = getTransport();
  const mirrorAddress = loadMirrorAddress();
  const publicClient = createShieldedPublicClient({ chain, transport });
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const walletClient: any = await createShieldedWalletClient({
    account: privateKeyToAccount(deployerKey as Hex),
    chain,
    transport,
  });

  const mirror = new BracketMirrorAdminClient(
    publicClient as any,
    walletClient,
    mirrorAddress,
  );

  console.log(`mirror contract: ${mirrorAddress}`);
  console.log(`wallet: ${walletClient.account.address}`);

  // Check for existing on-chain.json
  const onChainPath = resolve(cacheDir, "on-chain.json");
  let onChain: OnChainOutput | null = null;

  if (existsSync(onChainPath)) {
    onChain = JSON.parse(readFileSync(onChainPath, "utf-8"));
    console.log(
      `found existing on-chain.json: mirror id=${onChain!.mirror.id}, ${Object.keys(onChain!.members).length} members`,
    );
  }

  if (!onChain) {
    // Create new mirror
    console.log(`creating mirror "${platform.slug}"...`);
    const txHash = await mirror.createMirror(platform.slug, platform.slug);
    console.log(`  createMirror tx: ${txHash}`);

    // Wait for receipt
    const receipt = await publicClient.waitForTransactionReceipt({
      hash: txHash,
    });
    console.log(`  confirmed in block ${receipt.blockNumber}`);

    // Get mirror ID by slug
    const mirrorId = await mirror.getMirrorBySlug(platform.slug);
    console.log(`  mirror id: ${mirrorId}`);

    onChain = {
      mirror: { slug: platform.slug, id: Number(mirrorId) },
      members: {},
    };

    // Add all entries
    for (let i = 0; i < platform.entries.length; i++) {
      const entry = platform.entries[i];
      const entrySlug = makeEntrySlug(entry);
      console.log(
        `  [${i + 1}/${platform.entries.length}] adding ${entry.name} (${entrySlug})...`,
      );

      const addTx = await mirror.addEntry(
        mirrorId,
        entry.bracket as `0x${string}`,
        entrySlug,
      );
      const addReceipt = await publicClient.waitForTransactionReceipt({
        hash: addTx,
      });
      console.log(`    tx: ${addTx} (block ${addReceipt.blockNumber})`);

      onChain.members[entry.team_id] = {
        mirrorEntryId: i,
        slug: entrySlug,
      };
    }
  } else {
    // Update existing mirror — diff entries
    const mirrorId = BigInt(onChain.mirror.id);
    const existingIds = new Set(Object.keys(onChain.members));

    for (let i = 0; i < platform.entries.length; i++) {
      const entry = platform.entries[i];
      const entrySlug = makeEntrySlug(entry);

      if (existingIds.has(entry.team_id)) {
        // Already on-chain — check if slug changed
        const existing = onChain.members[entry.team_id];
        if (existing.slug !== entrySlug) {
          console.log(
            `  updating slug for ${entry.name}: "${existing.slug}" → "${entrySlug}"`,
          );
          const tx = await mirror.updateEntrySlug(
            mirrorId,
            BigInt(existing.mirrorEntryId),
            entrySlug,
          );
          await publicClient.waitForTransactionReceipt({ hash: tx });
          onChain.members[entry.team_id].slug = entrySlug;
        }
      } else {
        // New entry
        const nextIndex = Object.keys(onChain.members).length;
        console.log(
          `  adding new entry: ${entry.name} (${entrySlug})...`,
        );
        const tx = await mirror.addEntry(
          mirrorId,
          entry.bracket as `0x${string}`,
          entrySlug,
        );
        await publicClient.waitForTransactionReceipt({ hash: tx });
        onChain.members[entry.team_id] = {
          mirrorEntryId: nextIndex,
          slug: entrySlug,
        };
      }
    }
  }

  // Write on-chain.json
  writeFileSync(onChainPath, JSON.stringify(onChain, null, 2) + "\n");
  console.log(`wrote ${onChainPath}`);
  console.log("done!");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});

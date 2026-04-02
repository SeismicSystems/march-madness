/**
 * Migration script: snapshot V1 MarchMadness entries, reverse bracket bit ordering,
 * and import corrected brackets into MarchMadnessV2.
 *
 * Steps 3 & 4 of the encoding migration plan (#251).
 *
 * Usage:
 *   bun run src/migrate-entries.ts \
 *     --old-mm  0x...   # V1 MarchMadness address
 *     --new-mm  0x...   # V2 MarchMadnessV2 address
 *     --api-url http://localhost:3000   # server API base URL (for entry list)
 *     --rpc     http://localhost:8545   # RPC endpoint
 *     --private-key 0x...              # owner private key
 *     --dry-run                        # print manifest, skip on-chain writes
 *     --batch-size 50                  # entries per tx (default 50)
 *     --out scripts/migration/manifest-<ts>.json
 *
 * Output:
 *   - Prints a manifest table to stdout
 *   - Writes manifest JSON to --out (or scripts/migration/manifest-<ts>.json by default)
 *
 * Notes:
 *   - Idempotent: already-imported accounts are skipped by the V2 contract.
 *   - Only reverses brackets whose sentinel bit is set. Zero/invalid brackets are skipped
 *     with a warning.
 *   - Tags are imported individually after all brackets are imported.
 */

import { writeFileSync, mkdirSync } from "fs";
import { resolve } from "path";
import {
  http,
  createPublicClient,
  createWalletClient,
  type Address,
  type Hex,
} from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { sanvil } from "seismic-viem";
import type { EntryIndex } from "@march-madness/client";

// ── CLI Arg Parsing ───────────────────────────────────────────────────

interface CliArgs {
  oldMm: Address;
  newMm: Address;
  apiUrl: string;
  rpcUrl: string;
  privateKey: Hex;
  dryRun: boolean;
  batchSize: number;
  outPath: string;
}

function parseArgs(): CliArgs {
  const args = process.argv.slice(2);
  let oldMm: string | undefined;
  let newMm: string | undefined;
  let apiUrl = "http://localhost:3000";
  let rpcUrl = "http://localhost:8545";
  let privateKey: string | undefined;
  let dryRun = false;
  let batchSize = 50;
  let outPath: string | undefined;

  for (let i = 0; i < args.length; i++) {
    switch (args[i]) {
      case "--old-mm":
        oldMm = args[++i];
        break;
      case "--new-mm":
        newMm = args[++i];
        break;
      case "--api-url":
        apiUrl = args[++i];
        break;
      case "--rpc":
        rpcUrl = args[++i];
        break;
      case "--private-key":
        privateKey = args[++i];
        break;
      case "--dry-run":
        dryRun = true;
        break;
      case "--batch-size":
        batchSize = parseInt(args[++i], 10);
        break;
      case "--out":
        outPath = args[++i];
        break;
    }
  }

  if (!oldMm) {
    console.error("Error: --old-mm is required");
    process.exit(1);
  }
  if (!newMm) {
    console.error("Error: --new-mm is required");
    process.exit(1);
  }
  if (!dryRun && !privateKey) {
    console.error("Error: --private-key is required unless --dry-run");
    process.exit(1);
  }

  const tsNow = Math.floor(Date.now() / 1000);
  const defaultOut = resolve(
    import.meta.dir,
    `../../../scripts/migration/manifest-${tsNow}.json`
  );

  return {
    oldMm: oldMm as Address,
    newMm: newMm as Address,
    apiUrl,
    rpcUrl,
    privateKey: (privateKey ?? "0x0") as Hex,
    dryRun,
    batchSize,
    outPath: outPath ?? defaultOut,
  };
}

// ── Bracket Bit Reversal ──────────────────────────────────────────────

/**
 * Reverse the 63 non-sentinel game bits while preserving the sentinel (MSB).
 *
 * Legacy encoding: game 0 → bit 62 (MSB-1), game 62 → bit 0 (LSB).
 * Contract-correct: game 0 → bit 0 (LSB), game 62 → bit 62 (MSB-1).
 *
 * TypeScript port of `reverse_game_bits` in crates/seismic-march-madness/src/scoring.rs.
 */
function reverseLegacyBracket(hex: `0x${string}`): `0x${string}` {
  const v = BigInt(hex);
  const sentinel = v & 0x8000_0000_0000_0000n;
  const gameBits = v & 0x7fff_ffff_ffff_ffffn;
  let reversed = 0n;
  for (let i = 0; i < 63; i++) {
    reversed |= ((gameBits >> BigInt(i)) & 1n) << BigInt(62 - i);
  }
  const result = sentinel | reversed;
  return `0x${result.toString(16).padStart(16, "0")}` as `0x${string}`;
}

// ── MarchMadnessV2 ABI (migration surface only) ───────────────────────

const MarchMadnessV2Abi = [
  {
    name: "batchImportEntries",
    type: "function",
    stateMutability: "nonpayable",
    inputs: [
      { name: "accounts", type: "address[]" },
      { name: "bracketList", type: "bytes8[]" },
    ],
    outputs: [],
  },
  {
    name: "importTag",
    type: "function",
    stateMutability: "nonpayable",
    inputs: [
      { name: "account", type: "address" },
      { name: "tag", type: "string" },
    ],
    outputs: [],
  },
  {
    name: "hasEntry",
    type: "function",
    stateMutability: "view",
    inputs: [{ name: "account", type: "address" }],
    outputs: [{ type: "bool" }],
  },
  {
    name: "getBracket",
    type: "function",
    stateMutability: "view",
    inputs: [{ name: "account", type: "address" }],
    outputs: [{ type: "bytes8" }],
  },
] as const;

// ── V1 MarchMadness read ABI (getBracket + getTag) ────────────────────

const MarchMadnessV1ReadAbi = [
  {
    name: "getBracket",
    type: "function",
    stateMutability: "view",
    inputs: [{ name: "account", type: "address" }],
    outputs: [{ type: "bytes8" }],
  },
  {
    name: "getTag",
    type: "function",
    stateMutability: "view",
    inputs: [{ name: "account", type: "address" }],
    outputs: [{ type: "string" }],
  },
] as const;

// ── Manifest Types ────────────────────────────────────────────────────

interface ManifestEntry {
  address: string;
  old_bracket: string;
  new_bracket: string;
  tag: string | null;
  skipped?: string; // reason if skipped
}

interface Manifest {
  timestamp: string;
  old_contract: string;
  new_contract: string;
  api_url: string;
  dry_run: boolean;
  total_entries: number;
  imported: number;
  skipped: number;
  entries: ManifestEntry[];
}

// ── Main ──────────────────────────────────────────────────────────────

async function main() {
  const args = parseArgs();

  console.log("=== March Madness V1 → V2 Entry Migration ===");
  console.log(`  V1 contract: ${args.oldMm}`);
  console.log(`  V2 contract: ${args.newMm}`);
  console.log(`  API:         ${args.apiUrl}`);
  console.log(`  RPC:         ${args.rpcUrl}`);
  console.log(`  Batch size:  ${args.batchSize}`);
  console.log(`  Dry run:     ${args.dryRun}`);
  console.log(`  Output:      ${args.outPath}`);
  console.log("");

  // ── Fetch entry list from server API ──────────────────────────────
  console.log("Fetching entry index from server...");
  const res = await fetch(`${args.apiUrl}/entries`);
  if (!res.ok) {
    throw new Error(
      `GET /entries failed: ${res.status} ${res.statusText}. Is the server running?`
    );
  }
  const entryIndex: EntryIndex = await res.json();
  const addresses = Object.keys(entryIndex) as Address[];
  console.log(`  Found ${addresses.length} addresses in index.`);

  // ── Set up viem clients ────────────────────────────────────────────
  const transport = http(args.rpcUrl);
  const publicClient = createPublicClient({ transport, chain: sanvil });

  const walletClient = args.dryRun
    ? null
    : createWalletClient({
        transport,
        chain: sanvil,
        account: privateKeyToAccount(args.privateKey),
      });

  // ── Build migration manifest ───────────────────────────────────────
  console.log("Reading on-chain brackets from V1 contract...");
  const manifestEntries: ManifestEntry[] = [];
  let skipCount = 0;

  for (const addr of addresses) {
    // Read old bracket from V1 (after deadline, public read)
    let oldBracket: `0x${string}`;
    try {
      oldBracket = (await publicClient.readContract({
        address: args.oldMm,
        abi: MarchMadnessV1ReadAbi,
        functionName: "getBracket",
        args: [addr],
      })) as `0x${string}`;
    } catch (err) {
      const entry: ManifestEntry = {
        address: addr,
        old_bracket: "error",
        new_bracket: "error",
        tag: null,
        skipped: `getBracket failed: ${String(err)}`,
      };
      manifestEntries.push(entry);
      skipCount++;
      continue;
    }

    // Validate sentinel — skip zero/invalid brackets
    const oldBig = BigInt(oldBracket);
    if ((oldBig & 0x8000_0000_0000_0000n) === 0n) {
      const entry: ManifestEntry = {
        address: addr,
        old_bracket: oldBracket,
        new_bracket: oldBracket,
        tag: null,
        skipped: "no sentinel bit — bracket not submitted or invalid",
      };
      manifestEntries.push(entry);
      skipCount++;
      continue;
    }

    // Reverse the 63 game bits
    const newBracket = reverseLegacyBracket(oldBracket);

    // Read tag
    let tag: string | null = null;
    try {
      const rawTag = (await publicClient.readContract({
        address: args.oldMm,
        abi: MarchMadnessV1ReadAbi,
        functionName: "getTag",
        args: [addr],
      })) as string;
      tag = rawTag.length > 0 ? rawTag : null;
    } catch {
      // tags are optional; ignore errors
    }

    // Also use tag from index if on-chain tag is missing
    if (!tag && entryIndex[addr]?.name) {
      tag = entryIndex[addr].name ?? null;
    }

    manifestEntries.push({
      address: addr,
      old_bracket: oldBracket,
      new_bracket: newBracket,
      tag,
    });
  }

  const toImport = manifestEntries.filter((e) => !e.skipped);
  console.log(`  ${toImport.length} entries to import, ${skipCount} skipped.`);

  // ── Print manifest table ───────────────────────────────────────────
  console.log("\nManifest:");
  console.log(
    "  Address                                     | Old bracket        | New bracket        | Tag"
  );
  console.log("  " + "-".repeat(100));
  for (const e of manifestEntries) {
    const tag = e.tag ?? "(none)";
    const status = e.skipped ? `SKIP: ${e.skipped}` : tag;
    console.log(
      `  ${e.address.padEnd(43)} | ${e.old_bracket.padEnd(
        18
      )} | ${e.new_bracket.padEnd(18)} | ${status}`
    );
  }
  console.log("");

  if (args.dryRun) {
    console.log("Dry run complete — no on-chain writes performed.");
  } else {
    // ── Batch-import entries ─────────────────────────────────────────
    const batches: ManifestEntry[][] = [];
    for (let i = 0; i < toImport.length; i += args.batchSize) {
      batches.push(toImport.slice(i, i + args.batchSize));
    }

    console.log(
      `Importing ${toImport.length} entries in ${batches.length} batch(es)...`
    );
    for (let b = 0; b < batches.length; b++) {
      const batch = batches[b];
      const accounts = batch.map((e) => e.address as Address);
      const brackets = batch.map((e) => e.new_bracket as `0x${string}`);

      const hash = await walletClient!.writeContract({
        address: args.newMm,
        abi: MarchMadnessV2Abi,
        functionName: "batchImportEntries",
        args: [accounts, brackets],
      });
      await publicClient.waitForTransactionReceipt({ hash });
      console.log(`  Batch ${b + 1}/${batches.length} confirmed: ${hash}`);
    }

    // ── Import tags ──────────────────────────────────────────────────
    const withTags = toImport.filter((e) => e.tag);
    console.log(`\nImporting ${withTags.length} tag(s)...`);
    for (const e of withTags) {
      const hash = await walletClient!.writeContract({
        address: args.newMm,
        abi: MarchMadnessV2Abi,
        functionName: "importTag",
        args: [e.address as Address, e.tag!],
      });
      await publicClient.waitForTransactionReceipt({ hash });
      console.log(`  Tag for ${e.address}: "${e.tag}" — ${hash}`);
    }

    console.log("\nImport complete.");
  }

  // ── Write manifest JSON ────────────────────────────────────────────
  const manifest: Manifest = {
    timestamp: new Date().toISOString(),
    old_contract: args.oldMm,
    new_contract: args.newMm,
    api_url: args.apiUrl,
    dry_run: args.dryRun,
    total_entries: addresses.length,
    imported: toImport.length,
    skipped: skipCount,
    entries: manifestEntries,
  };

  mkdirSync(resolve(args.outPath, ".."), { recursive: true });
  writeFileSync(args.outPath, JSON.stringify(manifest, null, 2));
  console.log(`\nManifest written to: ${args.outPath}`);
}

main().catch((err) => {
  console.error("Fatal error:", err);
  process.exit(1);
});

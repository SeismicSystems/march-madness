/**
 * Migration script: V1 MarchMadness + BracketGroups → V2.
 *
 * Steps 3 & 4 of the encoding migration plan (#251).
 *
 * - Enumerates entries from on-chain BracketSubmitted events (no server API needed)
 * - Derives V1/V2 MarchMadness addresses from the BracketGroups contracts
 * - Reverses legacy bracket bit encoding before import
 * - Migrates groups and members via BracketGroupsV2
 *
 * Usage:
 *   bun run src/migrate-entries.ts \
 *     --old-bg      0x...          # V1 BracketGroups address
 *     --new-bg      0x...          # V2 BracketGroupsV2 address
 *     --rpc         http://...     # RPC endpoint (default: http://localhost:8545)
 *     --private-key 0x...          # owner private key (not needed with --dry-run)
 *     --from-block  0              # start block for event scanning (default: 0)
 *     --batch-size  50             # entries/members per tx (default: 50)
 *     --skip-entries               # skip entry migration, only migrate groups
 *     --skip-groups                # skip group migration, only migrate entries
 *     --dry-run                    # print manifest, skip on-chain writes
 *     --out         path/to/manifest.json
 *
 * Notes:
 *   - Idempotent: V2 contracts silently skip already-imported accounts.
 *   - Only reverses brackets with the sentinel bit set. Zero/invalid brackets are skipped.
 *   - Tags are imported individually after all brackets are imported.
 *   - Group IDs in V2 may differ from V1; the manifest records the mapping.
 */

import { writeFileSync, mkdirSync } from "fs";
import { resolve } from "path";
import {
  http,
  createPublicClient,
  createWalletClient,
  parseEventLogs,
  type Address,
  type Hex,
} from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { sanvil } from "seismic-viem";

// ── CLI Arg Parsing ───────────────────────────────────────────────────

interface CliArgs {
  oldBg: Address;
  newBg: Address;
  rpcUrl: string;
  privateKey: Hex;
  fromBlock: bigint;
  batchSize: number;
  skipEntries: boolean;
  skipGroups: boolean;
  dryRun: boolean;
  outPath: string;
}

function parseArgs(): CliArgs {
  const args = process.argv.slice(2);
  let oldBg: string | undefined;
  let newBg: string | undefined;
  let rpcUrl = "http://localhost:8545";
  let privateKey: string | undefined;
  let fromBlock = 0n;
  let batchSize = 50;
  let skipEntries = false;
  let skipGroups = false;
  let dryRun = false;
  let outPath: string | undefined;

  for (let i = 0; i < args.length; i++) {
    switch (args[i]) {
      case "--old-bg":       oldBg = args[++i]; break;
      case "--new-bg":       newBg = args[++i]; break;
      case "--rpc":          rpcUrl = args[++i]; break;
      case "--private-key":  privateKey = args[++i]; break;
      case "--from-block":   fromBlock = BigInt(args[++i]); break;
      case "--batch-size":   batchSize = parseInt(args[++i], 10); break;
      case "--skip-entries": skipEntries = true; break;
      case "--skip-groups":  skipGroups = true; break;
      case "--dry-run":      dryRun = true; break;
      case "--out":          outPath = args[++i]; break;
    }
  }

  if (!oldBg) { console.error("Error: --old-bg is required"); process.exit(1); }
  if (!newBg) { console.error("Error: --new-bg is required"); process.exit(1); }
  if (!dryRun && !privateKey) {
    console.error("Error: --private-key is required unless --dry-run");
    process.exit(1);
  }

  const tsNow = Math.floor(Date.now() / 1000);
  return {
    oldBg: oldBg as Address,
    newBg: newBg as Address,
    rpcUrl,
    privateKey: (privateKey ?? "0x0") as Hex,
    fromBlock,
    batchSize,
    skipEntries,
    skipGroups,
    dryRun,
    outPath: outPath ?? resolve(import.meta.dir, `../../../scripts/migration/manifest-${tsNow}.json`),
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
  return `0x${(sentinel | reversed).toString(16).padStart(16, "0")}` as `0x${string}`;
}

// ── ABIs ──────────────────────────────────────────────────────────────

const BracketGroupsReadAbi = [
  {
    name: "marchMadness",
    type: "function",
    stateMutability: "view",
    inputs: [],
    outputs: [{ type: "address" }],
  },
  {
    name: "getGroup",
    type: "function",
    stateMutability: "view",
    inputs: [{ name: "groupId", type: "uint32" }],
    outputs: [{
      type: "tuple",
      components: [
        { name: "slug", type: "string" },
        { name: "displayName", type: "string" },
        { name: "creator", type: "address" },
        { name: "entryCount", type: "uint32" },
        { name: "entryFee", type: "uint256" },
        { name: "hasPassword", type: "bool" },
      ],
    }],
  },
  {
    name: "getMembers",
    type: "function",
    stateMutability: "view",
    inputs: [{ name: "groupId", type: "uint32" }],
    outputs: [{
      type: "tuple[]",
      components: [
        { name: "addr", type: "address" },
        { name: "name", type: "string" },
        { name: "score", type: "uint256" },
        { name: "isScored", type: "bool" },
      ],
    }],
  },
  {
    name: "GroupCreated",
    type: "event",
    inputs: [
      { name: "groupId", type: "uint32", indexed: true },
      { name: "slug", type: "string", indexed: false },
      { name: "displayName", type: "string", indexed: false },
      { name: "creator", type: "address", indexed: false },
      { name: "hasPassword", type: "bool", indexed: false },
    ],
  },
] as const;

const BracketGroupsV2WriteAbi = [
  {
    name: "importGroup",
    type: "function",
    stateMutability: "nonpayable",
    inputs: [
      { name: "slug", type: "string" },
      { name: "displayName", type: "string" },
      { name: "entryFee", type: "uint256" },
      { name: "creator", type: "address" },
    ],
    outputs: [{ name: "groupId", type: "uint32" }],
  },
  {
    name: "batchImportMembers",
    type: "function",
    stateMutability: "payable",
    inputs: [
      { name: "groupId", type: "uint32" },
      { name: "addrs", type: "address[]" },
      { name: "names", type: "string[]" },
    ],
    outputs: [],
  },
] as const;

const MarchMadnessReadAbi = [
  {
    name: "entryFee",
    type: "function",
    stateMutability: "view",
    inputs: [],
    outputs: [{ type: "uint256" }],
  },
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
  {
    name: "BracketSubmitted",
    type: "event",
    inputs: [{ name: "account", type: "address", indexed: true }],
  },
] as const;

const MarchMadnessV2WriteAbi = [
  {
    name: "batchImportEntries",
    type: "function",
    stateMutability: "payable",
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
] as const;

// ── Manifest Types ────────────────────────────────────────────────────

interface ManifestEntry {
  address: string;
  old_bracket: string;
  new_bracket: string;
  tag: string | null;
  skipped?: string;
}

interface ManifestGroup {
  v1_group_id: number;
  v2_group_id: number | null;
  slug: string;
  display_name: string;
  creator: string;
  entry_fee: string;
  member_count: number;
  skipped?: string;
}

interface Manifest {
  timestamp: string;
  old_mm: string;
  new_mm: string;
  old_bg: string;
  new_bg: string;
  dry_run: boolean;
  entries: { total: number; imported: number; skipped: number; records: ManifestEntry[] };
  groups: { total: number; imported: number; skipped: number; records: ManifestGroup[] };
}

// ── Main ──────────────────────────────────────────────────────────────

async function main() {
  const args = parseArgs();

  const transport = http(args.rpcUrl);
  const publicClient = createPublicClient({ transport, chain: sanvil });
  const walletClient = args.dryRun
    ? null
    : createWalletClient({ transport, chain: sanvil, account: privateKeyToAccount(args.privateKey) });

  // ── Derive MM addresses from BG contracts ─────────────────────────
  const [oldMm, newMm] = await Promise.all([
    publicClient.readContract({ address: args.oldBg, abi: BracketGroupsReadAbi, functionName: "marchMadness" }),
    publicClient.readContract({ address: args.newBg, abi: BracketGroupsReadAbi, functionName: "marchMadness" }),
  ]);

  console.log("=== March Madness V1 → V2 Migration ===");
  console.log(`  V1 MM:        ${oldMm}`);
  console.log(`  V2 MM:        ${newMm}`);
  console.log(`  V1 BG:        ${args.oldBg}`);
  console.log(`  V2 BG:        ${args.newBg}`);
  console.log(`  RPC:          ${args.rpcUrl}`);
  console.log(`  From block:   ${args.fromBlock}`);
  console.log(`  Batch size:   ${args.batchSize}`);
  console.log(`  Skip entries: ${args.skipEntries}`);
  console.log(`  Skip groups:  ${args.skipGroups}`);
  console.log(`  Dry run:      ${args.dryRun}`);
  console.log(`  Output:       ${args.outPath}`);
  console.log("");

  const entryFee = await publicClient.readContract({
    address: newMm,
    abi: MarchMadnessReadAbi,
    functionName: "entryFee",
  });

  // ── Entry migration ───────────────────────────────────────────────
  const entryRecords: ManifestEntry[] = [];
  let entrySkipCount = 0;

  if (!args.skipEntries) {
    console.log("Scanning BracketSubmitted events...");
    const submitLogs = await publicClient.getLogs({
      address: oldMm,
      event: MarchMadnessReadAbi[3],
      fromBlock: args.fromBlock,
      toBlock: "latest",
    });
    // Deduplicate: keep latest submission per address (re-submissions overwrite)
    const addressSet = new Set<Address>();
    for (const log of submitLogs) {
      if (log.args.account) addressSet.add(log.args.account);
    }
    const addresses = [...addressSet];
    console.log(`  Found ${addresses.length} unique submitters.\n`);

    console.log("Reading on-chain brackets from V1...");
    for (const addr of addresses) {
      let oldBracket: `0x${string}`;
      try {
        oldBracket = (await publicClient.readContract({
          address: oldMm,
          abi: MarchMadnessReadAbi,
          functionName: "getBracket",
          args: [addr],
        })) as `0x${string}`;
      } catch (err) {
        entryRecords.push({ address: addr, old_bracket: "error", new_bracket: "error", tag: null, skipped: `getBracket failed: ${String(err)}` });
        entrySkipCount++;
        continue;
      }

      if ((BigInt(oldBracket) & 0x8000_0000_0000_0000n) === 0n) {
        entryRecords.push({ address: addr, old_bracket: oldBracket, new_bracket: oldBracket, tag: null, skipped: "no sentinel bit" });
        entrySkipCount++;
        continue;
      }

      const newBracket = reverseLegacyBracket(oldBracket);

      let tag: string | null = null;
      try {
        const rawTag = (await publicClient.readContract({
          address: oldMm,
          abi: MarchMadnessReadAbi,
          functionName: "getTag",
          args: [addr],
        })) as string;
        tag = rawTag.length > 0 ? rawTag : null;
      } catch { /* tags are optional */ }

      entryRecords.push({ address: addr, old_bracket: oldBracket, new_bracket: newBracket, tag });
    }

    const toImport = entryRecords.filter((e) => !e.skipped);
    console.log(`  ${toImport.length} to import, ${entrySkipCount} skipped.\n`);

    if (!args.dryRun && toImport.length > 0) {
      const batches: ManifestEntry[][] = [];
      for (let i = 0; i < toImport.length; i += args.batchSize) {
        batches.push(toImport.slice(i, i + args.batchSize));
      }
      console.log(`Importing ${toImport.length} entries in ${batches.length} batch(es)...`);
      for (let b = 0; b < batches.length; b++) {
        const batch = batches[b];
        const accounts = batch.map((e) => e.address as Address);
        const brackets = batch.map((e) => e.new_bracket as `0x${string}`);
        const value = BigInt(batch.length) * entryFee;
        const { request } = await publicClient.simulateContract({
          address: newMm,
          abi: MarchMadnessV2WriteAbi,
          functionName: "batchImportEntries",
          args: [accounts, brackets],
          value,
          account: walletClient!.account,
        });
        const hash = await walletClient!.writeContract(request);
        await publicClient.waitForTransactionReceipt({ hash });
        console.log(`  Batch ${b + 1}/${batches.length} confirmed: ${hash}`);
      }

      const withTags = toImport.filter((e) => e.tag);
      if (withTags.length > 0) {
        console.log(`\nImporting ${withTags.length} tag(s)...`);
        for (const e of withTags) {
          const { request } = await publicClient.simulateContract({
            address: newMm,
            abi: MarchMadnessV2WriteAbi,
            functionName: "importTag",
            args: [e.address as Address, e.tag!],
            account: walletClient!.account,
          });
          const hash = await walletClient!.writeContract(request);
          await publicClient.waitForTransactionReceipt({ hash });
          console.log(`  ${e.address}: "${e.tag}" — ${hash}`);
        }
      }
    }
  }

  // ── Group migration ───────────────────────────────────────────────
  const groupRecords: ManifestGroup[] = [];
  let groupSkipCount = 0;

  if (!args.skipGroups) {
    console.log("Scanning GroupCreated events...");
    const groupLogs = await publicClient.getLogs({
      address: args.oldBg,
      event: BracketGroupsReadAbi[3],
      fromBlock: args.fromBlock,
      toBlock: "latest",
    });
    const groupIds = groupLogs
      .map((log) => log.args.groupId)
      .filter((id): id is number => id !== undefined);
    console.log(`  Found ${groupIds.length} groups.\n`);

    for (const v1GroupId of groupIds) {
      let group: { slug: string; displayName: string; creator: Address; entryCount: number; entryFee: bigint; hasPassword: boolean };
      try {
        group = (await publicClient.readContract({
          address: args.oldBg,
          abi: BracketGroupsReadAbi,
          functionName: "getGroup",
          args: [v1GroupId],
        })) as typeof group;
      } catch (err) {
        groupRecords.push({ v1_group_id: v1GroupId, v2_group_id: null, slug: "?", display_name: "?", creator: "?", entry_fee: "?", member_count: 0, skipped: `getGroup failed: ${String(err)}` });
        groupSkipCount++;
        continue;
      }

      const members = (await publicClient.readContract({
        address: args.oldBg,
        abi: BracketGroupsReadAbi,
        functionName: "getMembers",
        args: [v1GroupId],
      })) as Array<{ addr: Address; name: string; score: bigint; isScored: boolean }>;

      if (args.dryRun) {
        groupRecords.push({ v1_group_id: v1GroupId, v2_group_id: null, slug: group.slug, display_name: group.displayName, creator: group.creator, entry_fee: group.entryFee.toString(), member_count: members.length });
        continue;
      }

      // Import the group
      const { request: importGroupReq, result: v2GroupId } = await publicClient.simulateContract({
        address: args.newBg,
        abi: BracketGroupsV2WriteAbi,
        functionName: "importGroup",
        args: [group.slug, group.displayName, group.entryFee, group.creator],
        account: walletClient!.account,
      });
      const importGroupHash = await walletClient!.writeContract(importGroupReq);
      await publicClient.waitForTransactionReceipt({ hash: importGroupHash });
      console.log(`  Group "${group.slug}" (V1 id=${v1GroupId} → V2 id=${v2GroupId}): ${importGroupHash}`);

      // Batch-import members
      if (members.length > 0) {
        const addrs = members.map((m) => m.addr);
        const names = members.map((m) => m.name);
        for (let i = 0; i < members.length; i += args.batchSize) {
          const batchAddrs = addrs.slice(i, i + args.batchSize);
          const batchNames = names.slice(i, i + args.batchSize);
          const value = BigInt(batchAddrs.length) * group.entryFee;
          const { request: membersReq } = await publicClient.simulateContract({
            address: args.newBg,
            abi: BracketGroupsV2WriteAbi,
            functionName: "batchImportMembers",
            args: [v2GroupId, batchAddrs, batchNames],
            value,
            account: walletClient!.account,
          });
          const hash = await walletClient!.writeContract(membersReq);
          await publicClient.waitForTransactionReceipt({ hash });
          console.log(`    Members ${i + 1}–${i + batchAddrs.length}: ${hash}`);
        }
      }

      groupRecords.push({ v1_group_id: v1GroupId, v2_group_id: v2GroupId, slug: group.slug, display_name: group.displayName, creator: group.creator, entry_fee: group.entryFee.toString(), member_count: members.length });
    }

    console.log(`\n  ${groupRecords.filter((g) => !g.skipped).length} groups imported, ${groupSkipCount} skipped.`);
  }

  if (args.dryRun) console.log("\nDry run complete — no on-chain writes performed.");

  // ── Write manifest ────────────────────────────────────────────────
  const toImportEntries = entryRecords.filter((e) => !e.skipped);
  const toImportGroups = groupRecords.filter((g) => !g.skipped);
  const manifest: Manifest = {
    timestamp: new Date().toISOString(),
    old_mm: oldMm,
    new_mm: newMm,
    old_bg: args.oldBg,
    new_bg: args.newBg,
    dry_run: args.dryRun,
    entries: { total: entryRecords.length, imported: toImportEntries.length, skipped: entrySkipCount, records: entryRecords },
    groups: { total: groupRecords.length, imported: toImportGroups.length, skipped: groupSkipCount, records: groupRecords },
  };

  mkdirSync(resolve(args.outPath, ".."), { recursive: true });
  writeFileSync(args.outPath, JSON.stringify(manifest, null, 2));
  console.log(`\nManifest written to: ${args.outPath}`);
}

main().catch((err) => {
  console.error("Fatal error:", err);
  process.exit(1);
});

/**
 * Test utilities for March Madness integration tests.
 * Helpers for random brackets, contract deployment via sforge, and anvil account management.
 */

import { readFileSync } from "fs";
import { resolve } from "path";
import { execSync } from "child_process";
import {
  type Address,
  type Hex,
  type Abi,
  parseEther,
  http,
  createPublicClient,
} from "viem";
import { privateKeyToAccount } from "viem/accounts";
import {
  createShieldedWalletClient,
  createShieldedPublicClient,
  sanvil,
} from "seismic-viem";
import type { ShieldedPublicClient } from "seismic-viem";
import { encodeBracket } from "@march-madness/client";

// We use `any` for wallet client type to avoid chain-narrowing issues with viem generics.
// At runtime, the client is always created with `chain: sanvil`.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WalletClient = any;

// ── Paths ─────────────────────────────────────────────────────────────

const PROJECT_ROOT = resolve(import.meta.dir, "../../..");
const CONTRACTS_DIR = resolve(PROJECT_ROOT, "contracts");
const DATA_DIR = resolve(PROJECT_ROOT, "data");

// ── Contract ABI ──────────────────────────────────────────────────────

function loadContractArtifact(): { abi: Abi; bytecode: Hex } {
  const artifactPath = resolve(
    CONTRACTS_DIR,
    "out/MarchMadness.sol/MarchMadness.json",
  );
  const artifact = JSON.parse(readFileSync(artifactPath, "utf-8"));
  return {
    abi: artifact.abi as Abi,
    bytecode: artifact.bytecode.object as Hex,
  };
}

let _artifact: { abi: Abi; bytecode: Hex } | null = null;

export function getContractArtifact(): { abi: Abi; bytecode: Hex } {
  if (!_artifact) {
    _artifact = loadContractArtifact();
  }
  return _artifact;
}

export function getAbi(): Abi {
  return getContractArtifact().abi;
}

// ── Anvil Accounts (from data/anvil-accounts.json) ────────────────────

export interface AnvilAccount {
  address: Address;
  privateKey: Hex;
  label: string;
  index: number;
}

let _accounts: AnvilAccount[] | null = null;

export function getAnvilAccounts(): AnvilAccount[] {
  if (!_accounts) {
    const raw = JSON.parse(
      readFileSync(resolve(DATA_DIR, "anvil-accounts.json"), "utf-8"),
    );
    _accounts = raw.map(
      (
        a: { address: string; private_key: string; label: string },
        i: number,
      ) => ({
        address: a.address as Address,
        privateKey: a.private_key as Hex,
        label: a.label,
        index: i,
      }),
    );
  }
  return _accounts!;
}

/** Account #0 is the deployer/owner. */
export function getDeployerAccount(): AnvilAccount {
  return getAnvilAccounts()[0];
}

/** Accounts #1-9 are test bracket submitters. */
export function getPlayerAccounts(): AnvilAccount[] {
  return getAnvilAccounts().slice(1);
}

// ── Client Creation ───────────────────────────────────────────────────

const RPC_URL = process.env.RPC_URL || "http://localhost:8545";

export function getTransport() {
  return http(RPC_URL);
}

export async function createWallet(
  privateKey: Hex,
): Promise<WalletClient> {
  return await createShieldedWalletClient({
    account: privateKeyToAccount(privateKey),
    chain: sanvil,
    transport: getTransport(),
  });
}

export function createPublicClientInstance(): ShieldedPublicClient {
  return createShieldedPublicClient({
    chain: sanvil,
    transport: getTransport(),
  }) as ShieldedPublicClient;
}

// ── Random Bracket Generation ─────────────────────────────────────────

/**
 * Generate a random valid bracket (63 random booleans encoded to bytes8 hex).
 * MSB sentinel bit is always set.
 */
export function randomBracket(): `0x${string}` {
  const picks: boolean[] = [];
  for (let i = 0; i < 63; i++) {
    picks.push(Math.random() < 0.5);
  }
  return encodeBracket(picks);
}

/**
 * Generate a bracket where higher seeds always win.
 * Useful for deterministic testing.
 */
export function chalkyBracket(): `0x${string}` {
  const picks = Array(63).fill(true);
  return encodeBracket(picks);
}

// ── Contract Deployment via sforge script ─────────────────────────────

export interface DeployResult {
  contractAddress: Address;
  ownerAddress: Address;
  deadline: bigint;
}

/**
 * Deploy the MarchMadness contract using the sforge deploy script.
 *
 * Runs: cd contracts && mise run sforge -- script script/MarchMadnessLocal.s.sol \
 *         --rpc-url $RPC_URL --broadcast --private-key $DEPLOYER_KEY
 *
 * @param deadlineOffset - Seconds from now until submission deadline (default: 3600)
 * @returns Deployed contract address, owner, and deadline
 */
export async function deployContractViaSforge(
  deadlineOffset: number = 3600,
): Promise<DeployResult> {
  const deployer = getDeployerAccount();
  const rpcUrl = RPC_URL;

  const cmd = [
    "mise run sforge --",
    "script script/MarchMadnessLocal.s.sol",
    `--rpc-url ${rpcUrl}`,
    "--broadcast",
    `--private-key ${deployer.privateKey}`,
  ].join(" ");

  let output: string;
  try {
    output = execSync(cmd, {
      cwd: CONTRACTS_DIR,
      encoding: "utf-8",
      env: {
        ...process.env,
        DEADLINE_OFFSET: String(deadlineOffset),
      },
      timeout: 60_000,
    });
  } catch (err: any) {
    const stderr = err.stderr || "";
    const stdout = err.stdout || "";
    throw new Error(
      `sforge deploy failed.\nstdout: ${stdout}\nstderr: ${stderr}`,
    );
  }

  // Parse deployed address from sforge output
  // Expected line: "MarchMadness (local) deployed at: 0x..."
  const addressMatch = output.match(
    /deployed at:\s+(0x[0-9a-fA-F]{40})/,
  );
  if (!addressMatch) {
    throw new Error(
      `Could not parse contract address from sforge output:\n${output}`,
    );
  }

  const contractAddress = addressMatch[1] as Address;

  // Read the deadline from the contract
  const publicClient = createPublicClientInstance();
  const abi = getAbi();

  const deadline = (await publicClient.readContract({
    address: contractAddress,
    abi,
    functionName: "submissionDeadline",
  })) as bigint;

  return {
    contractAddress,
    ownerAddress: deployer.address,
    deadline,
  };
}

// ── Sanvil Process Management ─────────────────────────────────────────

/**
 * Spawn a sanvil process in the background.
 * Polls getChainId until the node is reachable (up to ~30s).
 * Returns a cleanup function to kill the process.
 */
export async function spawnSanvil(
  port: number = 8545,
): Promise<{ kill: () => void }> {
  const { spawn } = await import("child_process");

  const proc = spawn("sanvil", ["--port", String(port), "--silent"], {
    stdio: "ignore",
    detached: true,
  });

  // Unref so the parent process can exit without waiting for sanvil
  proc.unref();

  // Poll until sanvil is reachable
  const maxAttempts = 60;
  const pollInterval = 500;
  for (let i = 0; i < maxAttempts; i++) {
    try {
      const client = createPublicClientInstance();
      await client.getChainId();
      console.log(`sanvil started on port ${port} (pid: ${proc.pid})`);
      return {
        kill: () => {
          try {
            if (proc.pid) process.kill(proc.pid);
          } catch {
            // already dead
          }
        },
      };
    } catch {
      await new Promise((r) => setTimeout(r, pollInterval));
    }
  }

  // If we get here, sanvil never became reachable
  try {
    if (proc.pid) process.kill(proc.pid);
  } catch { /* ignore */ }
  throw new Error(`sanvil failed to start on port ${port} within ${(maxAttempts * pollInterval) / 1000}s`);
}

// ── Time Manipulation ─────────────────────────────────────────────────

/**
 * Fast-forward the local node's time by the given number of seconds.
 * Uses evm_increaseTime + evm_mine (standard anvil/sanvil JSON-RPC methods).
 */
export async function increaseTime(seconds: number): Promise<void> {
  const client = createPublicClient({
    chain: sanvil,
    transport: getTransport(),
  });

  await client.request({
    method: "evm_increaseTime" as any,
    params: [seconds] as any,
  });
  await client.request({
    method: "evm_mine" as any,
    params: [] as any,
  });
}

// ── Constants ─────────────────────────────────────────────────────────

export const ENTRY_FEE = parseEther("1");

// ── Connection Check ──────────────────────────────────────────────────

/**
 * Check if the local sanvil node is reachable.
 */
export async function isSanvilRunning(): Promise<boolean> {
  try {
    const client = createPublicClientInstance();
    await client.getChainId();
    return true;
  } catch {
    return false;
  }
}

// ── Fun Tag Names ─────────────────────────────────────────────────────

export const TAG_NAMES = [
  "Duke4Lyfe",
  "BracketBuster",
  "CinderellaStory",
  "MarchSadness",
  "ChalkCity",
  "UnderdogSZN",
  "BigDanceEnergy",
  "BubbleTeam",
  "FinalFourOrBust",
];

// Three-level client hierarchy for MarchMadness contract interaction.
//
// - PublicClient: transparent reads only (no wallet needed)
// - UserClient: extends public + shielded writes + signed reads
// - OwnerClient: extends user + owner-only functions
//
// Uses seismic-viem's getShieldedContract for shielded write/signed read support.
// Key pattern:
//   contract.write.xxx()  → shielded write (encrypted calldata)
//   contract.read.xxx()   → signed read (wallet-authenticated)
//   contract.tread.xxx()  → transparent read (public, no auth)
//   contract.twrite.xxx() → transparent write (unencrypted calldata)

import { getShieldedContract } from "seismic-viem";
import type { ShieldedWalletClient, ShieldedContract } from "seismic-viem";
import {
  type Address,
  type PublicClient as ViemPublicClient,
  type Hash,
  type Account,
  type Chain,
  type Transport,
  type GetContractReturnType,
  getContract,
  parseEther,
} from "viem";

import { MarchMadnessAbi } from "./abi.ts";

// ── Shielded contract type ──────────────────────────────────────────
type MarchMadnessContract = ShieldedContract<
  Transport,
  Address,
  typeof MarchMadnessAbi,
  Chain,
  Account,
  ShieldedWalletClient<Transport, Chain, Account>
>;

// ── Entry fee constant ──────────────────────────────────────────────
export const ENTRY_FEE = parseEther("1");

// ── Types ───────────────────────────────────────────────────────────

/** Options for read calls (e.g. specific block number). */
export type ReadOptions = {
  blockNumber?: bigint;
};

/** Options for write calls (gas overrides, nonce, etc). */
export type WriteOptions = {
  nonce?: number;
  gas?: bigint;
  gasPrice?: bigint;
};

// ── PublicClient ────────────────────────────────────────────────────

/**
 * Read-only client for MarchMadness contract. Uses transparent reads.
 * No wallet or signing needed — good for displaying public data.
 */
export class MarchMadnessPublicClient {
  protected readonly contractAddress: Address;
  protected readonly publicClient: ViemPublicClient;
  protected readonly contract: GetContractReturnType<
    typeof MarchMadnessAbi,
    ViemPublicClient
  >;

  constructor(publicClient: ViemPublicClient, contractAddress: Address) {
    this.publicClient = publicClient;
    this.contractAddress = contractAddress;
    this.contract = getContract({
      address: contractAddress,
      abi: MarchMadnessAbi,
      client: publicClient,
    });
  }

  /** Number of brackets submitted. */
  async getEntryCount(opts: ReadOptions = {}): Promise<number> {
    return this.contract.read.getEntryCount(opts);
  }

  /** Tournament results (bytes8). Returns 0x0000000000000000 if not yet posted. */
  async getResults(opts: ReadOptions = {}): Promise<`0x${string}`> {
    return this.contract.read.results(opts);
  }

  /** Submission deadline as unix timestamp (bigint). */
  async getSubmissionDeadline(opts: ReadOptions = {}): Promise<bigint> {
    return this.contract.read.submissionDeadline(opts);
  }

  /** Contract owner address. */
  async getOwner(opts: ReadOptions = {}): Promise<Address> {
    return this.contract.read.owner(opts);
  }

  /** Entry fee in wei (bigint). */
  async getEntryFee(opts: ReadOptions = {}): Promise<bigint> {
    return this.contract.read.entryFee(opts);
  }

  /** Read a bracket (after deadline — transparent read). */
  async getBracket(
    account: Address,
    opts: ReadOptions = {},
  ): Promise<`0x${string}`> {
    return this.contract.read.getBracket([account], opts);
  }

  /** Get score for an account. */
  async getScore(account: Address, opts: ReadOptions = {}): Promise<number> {
    return this.contract.read.getScore([account], opts);
  }

  /** Check if a bracket has been scored. */
  async getIsScored(
    account: Address,
    opts: ReadOptions = {},
  ): Promise<boolean> {
    return this.contract.read.getIsScored([account], opts);
  }

  /** Get display tag for an account. */
  async getTag(account: Address, opts: ReadOptions = {}): Promise<string> {
    return this.contract.read.getTag([account], opts);
  }

  /** Winning score (highest score among scored brackets). */
  async getWinningScore(opts: ReadOptions = {}): Promise<number> {
    return this.contract.read.winningScore(opts);
  }

  /** Number of winners (brackets with the winning score). */
  async getNumWinners(opts: ReadOptions = {}): Promise<bigint> {
    return this.contract.read.numWinners(opts);
  }

  /** Timestamp when results were posted (0 if not posted). */
  async getResultsPostedAt(opts: ReadOptions = {}): Promise<bigint> {
    return this.contract.read.resultsPostedAt(opts);
  }
}

// ── UserClient ─────────────────────────────────────────────────────

/**
 * Wallet-connected client for MarchMadness. Extends PublicClient with:
 * - Shielded writes (bracket submission/update)
 * - Signed reads (reading own bracket before deadline)
 * - Transparent writes (setTag, scoreBracket, collectWinnings)
 */
export class MarchMadnessUserClient extends MarchMadnessPublicClient {
  protected readonly walletClient: ShieldedWalletClient<
    Transport,
    Chain,
    Account
  >;
  protected readonly shieldedContract: MarchMadnessContract;

  constructor(
    publicClient: ViemPublicClient,
    walletClient: ShieldedWalletClient<Transport, Chain, Account>,
    contractAddress: Address,
  ) {
    super(publicClient, contractAddress);
    this.walletClient = walletClient;
    this.shieldedContract = getShieldedContract({
      abi: MarchMadnessAbi,
      address: contractAddress,
      client: walletClient,
    }) as unknown as MarchMadnessContract;
  }

  /** The wallet's address. */
  get account(): Address {
    return this.walletClient.account.address;
  }

  /**
   * Submit a shielded bracket with entry fee.
   * Uses shielded write (encrypted calldata) via contract.write.
   */
  async submitBracket(
    bracket: `0x${string}`,
    opts: WriteOptions = {},
  ): Promise<Hash> {
    return this.shieldedContract.write.submitBracket([bracket], {
      value: ENTRY_FEE,
      ...opts,
    });
  }

  /**
   * Update an already-submitted bracket (no additional fee).
   * Uses shielded write (encrypted calldata) via contract.write.
   */
  async updateBracket(
    bracket: `0x${string}`,
    opts: WriteOptions = {},
  ): Promise<Hash> {
    return this.shieldedContract.write.updateBracket([bracket], opts);
  }

  /**
   * Set or update a display tag. Uses transparent write since tags are public.
   */
  async setTag(tag: string, opts: WriteOptions = {}): Promise<Hash> {
    return this.shieldedContract.twrite.setTag([tag], opts);
  }

  /**
   * Read the caller's own bracket.
   * Before deadline: uses signed read (wallet-authenticated).
   * After deadline: uses transparent read (public).
   */
  async getMyBracket(opts: ReadOptions = {}): Promise<`0x${string}`> {
    const deadline = await this.getSubmissionDeadline(opts);
    const now = BigInt(Math.floor(Date.now() / 1000));

    if (now < deadline) {
      // Before deadline: signed read (only msg.sender == account can read)
      return this.shieldedContract.read.getBracket([this.account], opts);
    } else {
      // After deadline: transparent read (anyone can read)
      return this.getBracket(this.account, opts);
    }
  }

  /**
   * Score anyone's bracket against posted results.
   * Uses transparent write since scoring is public.
   */
  async scoreBracket(
    account: Address,
    opts: WriteOptions = {},
  ): Promise<Hash> {
    return this.shieldedContract.twrite.scoreBracket([account], opts);
  }

  /**
   * Collect winnings (if caller has the winning score).
   * Uses transparent write since payout is public.
   */
  async collectWinnings(opts: WriteOptions = {}): Promise<Hash> {
    return this.shieldedContract.twrite.collectWinnings(opts);
  }
}

// ── OwnerClient ────────────────────────────────────────────────────

/**
 * Owner-only client for MarchMadness. Extends UserClient with:
 * - submitResults: post tournament results (owner only, once)
 */
export class MarchMadnessOwnerClient extends MarchMadnessUserClient {
  /**
   * Post tournament results. Owner only, once, after submission deadline.
   * Results are unencrypted (bytes8, not sbytes8), so we use transparent write.
   */
  async submitResults(
    results: `0x${string}`,
    opts: WriteOptions = {},
  ): Promise<Hash> {
    return this.shieldedContract.twrite.submitResults([results], opts);
  }
}

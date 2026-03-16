// Typed client wrappers for BracketMirror contract interaction.
//
// - BracketMirrorPublicClient: transparent reads (no wallet needed)
// - BracketMirrorAdminClient: extends public + admin write methods

import {
  type Address,
  type PublicClient as ViemPublicClient,
  type Hash,
  type Account,
  type Chain,
  type Transport,
  type GetContractReturnType,
  getContract,
} from "viem";

import { BracketMirrorAbi } from "./abi-mirror.ts";
import type { ReadOptions, WriteOptions } from "./client.ts";

// ── Types ─────────────────────────────────────────────────────

/** On-chain Mirror struct. */
export interface MirrorData {
  slug: string;
  displayName: string;
  entryFee: number;
  entryCurrency: string;
  admin: Address;
}

/** On-chain MirrorEntry struct. */
export interface MirrorEntryData {
  bracket: `0x${string}`;
  slug: string;
}

// ── PublicClient ──────────────────────────────────────────────

/**
 * Read-only client for BracketMirror contract.
 */
export class BracketMirrorPublicClient {
  protected readonly contractAddress: Address;
  protected readonly publicClient: ViemPublicClient;
  protected readonly contract: GetContractReturnType<
    typeof BracketMirrorAbi,
    ViemPublicClient
  >;

  constructor(publicClient: ViemPublicClient, contractAddress: Address) {
    this.publicClient = publicClient;
    this.contractAddress = contractAddress;
    this.contract = getContract({
      address: contractAddress,
      abi: BracketMirrorAbi,
      client: publicClient,
    });
  }

  /** Get mirror by ID. */
  async getMirror(
    mirrorId: bigint,
    opts: ReadOptions = {},
  ): Promise<MirrorData> {
    return this.contract.read.getMirror([mirrorId], opts) as Promise<MirrorData>;
  }

  /** Get mirror ID by slug. */
  async getMirrorBySlug(
    slug: string,
    opts: ReadOptions = {},
  ): Promise<bigint> {
    return this.contract.read.getMirrorBySlug([slug], opts);
  }

  /** Get entry count for a mirror. */
  async getEntryCount(
    mirrorId: bigint,
    opts: ReadOptions = {},
  ): Promise<bigint> {
    return this.contract.read.getEntryCount([mirrorId], opts);
  }

  /** Get a single entry by index. */
  async getEntry(
    mirrorId: bigint,
    index: bigint,
    opts: ReadOptions = {},
  ): Promise<MirrorEntryData> {
    return this.contract.read.getEntry(
      [mirrorId, index],
      opts,
    ) as Promise<MirrorEntryData>;
  }

  /** Get entry by slug. */
  async getEntryBySlug(
    mirrorId: bigint,
    slug: string,
    opts: ReadOptions = {},
  ): Promise<MirrorEntryData> {
    return this.contract.read.getEntryBySlug(
      [mirrorId, slug],
      opts,
    ) as Promise<MirrorEntryData>;
  }

  /** Get all entries for a mirror. */
  async getEntries(
    mirrorId: bigint,
    opts: ReadOptions = {},
  ): Promise<MirrorEntryData[]> {
    return this.contract.read.getEntries(
      [mirrorId],
      opts,
    ) as Promise<MirrorEntryData[]>;
  }

  /** Get the next mirror ID. */
  async getNextMirrorId(opts: ReadOptions = {}): Promise<bigint> {
    return this.contract.read.nextMirrorId(opts);
  }
}

// ── AdminClient ───────────────────────────────────────────────

/**
 * Admin client for BracketMirror. Extends public with write methods.
 * All writes are transparent (no shielded data in Mirror).
 */
export class BracketMirrorAdminClient extends BracketMirrorPublicClient {
  protected readonly walletClient: {
    account: { address: Address };
    writeContract: (...args: unknown[]) => Promise<Hash>;
  };

  constructor(
    publicClient: ViemPublicClient,
    walletClient: { account: { address: Address }; writeContract: (...args: unknown[]) => Promise<Hash> },
    contractAddress: Address,
  ) {
    super(publicClient, contractAddress);
    this.walletClient = walletClient;
  }

  /** The wallet's address. */
  get account(): Address {
    return this.walletClient.account.address;
  }

  /** Create a new mirror. */
  async createMirror(
    slug: string,
    displayName: string,
    opts: WriteOptions = {},
  ): Promise<Hash> {
    return this.walletClient.writeContract({
      address: this.contractAddress,
      abi: BracketMirrorAbi,
      functionName: "createMirror",
      args: [slug, displayName],
      ...opts,
    } as never);
  }

  /** Set entry fee display info. */
  async setEntryFee(
    mirrorId: bigint,
    fee: number,
    currency: string,
    opts: WriteOptions = {},
  ): Promise<Hash> {
    return this.walletClient.writeContract({
      address: this.contractAddress,
      abi: BracketMirrorAbi,
      functionName: "setEntryFee",
      args: [mirrorId, fee, currency],
      ...opts,
    } as never);
  }

  /** Add a bracket entry to a mirror. */
  async addEntry(
    mirrorId: bigint,
    bracket: `0x${string}`,
    slug: string,
    opts: WriteOptions = {},
  ): Promise<Hash> {
    return this.walletClient.writeContract({
      address: this.contractAddress,
      abi: BracketMirrorAbi,
      functionName: "addEntry",
      args: [mirrorId, bracket, slug],
      ...opts,
    } as never);
  }

  /** Remove an entry from a mirror (swap-and-pop). */
  async removeEntry(
    mirrorId: bigint,
    entryIndex: bigint,
    opts: WriteOptions = {},
  ): Promise<Hash> {
    return this.walletClient.writeContract({
      address: this.contractAddress,
      abi: BracketMirrorAbi,
      functionName: "removeEntry",
      args: [mirrorId, entryIndex],
      ...opts,
    } as never);
  }

  /** Update bracket for an entry. */
  async updateBracket(
    mirrorId: bigint,
    entryIndex: bigint,
    bracket: `0x${string}`,
    opts: WriteOptions = {},
  ): Promise<Hash> {
    return this.walletClient.writeContract({
      address: this.contractAddress,
      abi: BracketMirrorAbi,
      functionName: "updateBracket",
      args: [mirrorId, entryIndex, bracket],
      ...opts,
    } as never);
  }

  /** Update entry slug. */
  async updateEntrySlug(
    mirrorId: bigint,
    entryIndex: bigint,
    slug: string,
    opts: WriteOptions = {},
  ): Promise<Hash> {
    return this.walletClient.writeContract({
      address: this.contractAddress,
      abi: BracketMirrorAbi,
      functionName: "updateEntrySlug",
      args: [mirrorId, entryIndex, slug],
      ...opts,
    } as never);
  }
}

// Typed client wrappers for BracketGroups contract interaction.
//
// - BracketGroupsPublicClient: transparent reads (no wallet needed)
// - BracketGroupsUserClient: extends public + shielded writes for password joins + transparent writes

import { getShieldedContract } from "seismic-viem";
import type { ShieldedWalletClient, ShieldedContract } from "seismic-viem";
import {
  type Address,
  type Hex,
  type PublicClient as ViemPublicClient,
  type Hash,
  type Account,
  type Chain,
  type Transport,
  type GetContractReturnType,
  getContract,
} from "viem";

import { BracketGroupsAbi } from "./abi-groups.ts";
import type { ReadOptions, WriteOptions } from "./client.ts";

// ── Types ─────────────────────────────────────────────────────

/** On-chain Group struct returned by getGroup / getGroupBySlug. */
export interface GroupData {
  slug: string;
  displayName: string;
  creator: Address;
  entryCount: number;
  entryFee: bigint;
  hasPassword: boolean;
}

/** On-chain Member struct returned by getMembers / getMember. */
export interface MemberData {
  addr: Address;
  name: string;
  score: number;
  isScored: boolean;
}

/** On-chain GroupPayout struct returned by payouts(). */
export interface GroupPayoutData {
  winningScore: number;
  numWinners: number;
  numScored: number;
}

// ── Shielded contract type ────────────────────────────────────
type BracketGroupsContract = ShieldedContract<
  Transport,
  Address,
  typeof BracketGroupsAbi,
  Chain,
  Account,
  ShieldedWalletClient<Transport, Chain, Account>
>;

// ── PublicClient ──────────────────────────────────────────────

/**
 * Read-only client for BracketGroups contract.
 */
export class BracketGroupsPublicClient {
  protected readonly contractAddress: Address;
  protected readonly publicClient: ViemPublicClient;
  protected readonly contract: GetContractReturnType<
    typeof BracketGroupsAbi,
    ViemPublicClient
  >;

  constructor(publicClient: ViemPublicClient, contractAddress: Address) {
    this.publicClient = publicClient;
    this.contractAddress = contractAddress;
    this.contract = getContract({
      address: contractAddress,
      abi: BracketGroupsAbi,
      client: publicClient,
    });
  }

  /** Get group by ID. */
  async getGroup(groupId: number, opts: ReadOptions = {}): Promise<GroupData> {
    return this.contract.read.getGroup([groupId], opts) as Promise<GroupData>;
  }

  /** Get group by slug. Returns [groupId, groupData]. */
  async getGroupBySlug(
    slug: string,
    opts: ReadOptions = {},
  ): Promise<[number, GroupData]> {
    return this.contract.read.getGroupBySlug([slug], opts) as Promise<
      [number, GroupData]
    >;
  }

  /** Get all members of a group. */
  async getMembers(
    groupId: number,
    opts: ReadOptions = {},
  ): Promise<MemberData[]> {
    return this.contract.read.getMembers([groupId], opts) as Promise<
      MemberData[]
    >;
  }

  /** Get a specific member by index. */
  async getMember(
    groupId: number,
    index: number,
    opts: ReadOptions = {},
  ): Promise<MemberData> {
    return this.contract.read.getMember(
      [groupId, index],
      opts,
    ) as Promise<MemberData>;
  }

  /** Check if an address is a member of a group. */
  async getIsMember(
    groupId: number,
    addr: Address,
    opts: ReadOptions = {},
  ): Promise<boolean> {
    return this.contract.read.getIsMember([groupId, addr], opts);
  }

  /** Check if an address is a member of a group (alias). */
  async isMemberOf(
    groupId: number,
    addr: Address,
    opts: ReadOptions = {},
  ): Promise<boolean> {
    return this.contract.read.isMemberOf([groupId, addr], opts);
  }

  /** Get payout info for a group. */
  async getPayouts(
    groupId: number,
    opts: ReadOptions = {},
  ): Promise<GroupPayoutData> {
    const result = await this.contract.read.payouts([groupId], opts);
    return result as unknown as GroupPayoutData;
  }

  /** Check if an address has collected winnings for a group. */
  async getHasCollectedWinnings(
    groupId: number,
    addr: Address,
    opts: ReadOptions = {},
  ): Promise<boolean> {
    return this.contract.read.hasCollectedWinnings([groupId, addr], opts);
  }

  /** Get the next group ID. */
  async getNextGroupId(opts: ReadOptions = {}): Promise<number> {
    return this.contract.read.nextGroupId(opts);
  }
}

// ── UserClient ────────────────────────────────────────────────

/**
 * Wallet-connected client for BracketGroups.
 * Extends public client with write methods (join, leave, create, score, collect).
 * Password-protected group joins use shielded writes (sbytes12).
 */
export class BracketGroupsUserClient extends BracketGroupsPublicClient {
  protected readonly walletClient: ShieldedWalletClient<
    Transport,
    Chain,
    Account
  >;
  protected readonly shieldedContract: BracketGroupsContract;

  constructor(
    publicClient: ViemPublicClient,
    walletClient: ShieldedWalletClient<Transport, Chain, Account>,
    contractAddress: Address,
  ) {
    super(publicClient, contractAddress);
    this.walletClient = walletClient;
    this.shieldedContract = getShieldedContract({
      abi: BracketGroupsAbi,
      address: contractAddress,
      client: walletClient,
    }) as unknown as BracketGroupsContract;
  }

  /** The wallet's address. */
  get account(): Address {
    return this.walletClient.account.address;
  }

  /** Create a public group (no password). */
  async createGroup(
    slug: string,
    displayName: string,
    entryFee: bigint,
    opts: WriteOptions = {},
  ): Promise<Hash> {
    return this.shieldedContract.twrite.createGroup(
      [slug, displayName, entryFee],
      opts,
    );
  }

  /** Create a password-protected group. Password is sbytes12 (shielded write). */
  async createGroupWithPassword(
    slug: string,
    displayName: string,
    entryFee: bigint,
    password: Hex,
    opts: WriteOptions = {},
  ): Promise<Hash> {
    return this.shieldedContract.write.createGroupWithPassword(
      [slug, displayName, entryFee, password],
      opts,
    );
  }

  /** Join a public group with a display name. Payable (entry fee). */
  async joinGroup(
    groupId: number,
    name: string,
    entryFee: bigint = 0n,
    opts: WriteOptions = {},
  ): Promise<Hash> {
    return this.shieldedContract.twrite.joinGroup([groupId, name], {
      value: entryFee,
      ...opts,
    });
  }

  /** Join a password-protected group. Uses shielded write for password (sbytes12). */
  async joinGroupWithPassword(
    groupId: number,
    password: Hex,
    name: string,
    entryFee: bigint = 0n,
    opts: WriteOptions = {},
  ): Promise<Hash> {
    return this.shieldedContract.write.joinGroupWithPassword(
      [groupId, password, name],
      { value: entryFee, ...opts },
    );
  }

  /** Leave a group. Only before submission deadline. Refunds entry fee. */
  async leaveGroup(
    groupId: number,
    opts: WriteOptions = {},
  ): Promise<Hash> {
    return this.shieldedContract.twrite.leaveGroup([groupId], opts);
  }

  /** Update display name in a group. */
  async editEntryName(
    groupId: number,
    name: string,
    opts: WriteOptions = {},
  ): Promise<Hash> {
    return this.shieldedContract.twrite.editEntryName(
      [groupId, name],
      opts,
    );
  }

  /** Score a group member's bracket. Anyone can call. */
  async scoreEntry(
    groupId: number,
    memberIndex: number,
    opts: WriteOptions = {},
  ): Promise<Hash> {
    return this.shieldedContract.twrite.scoreEntry(
      [groupId, memberIndex],
      opts,
    );
  }

  /** Collect winnings for a group. */
  async collectWinnings(
    groupId: number,
    opts: WriteOptions = {},
  ): Promise<Hash> {
    return this.shieldedContract.twrite.collectWinnings([groupId], opts);
  }
}

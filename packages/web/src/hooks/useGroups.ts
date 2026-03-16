import { useCallback, useEffect, useMemo, useState } from "react";
import { useShieldedWallet } from "seismic-react";
import {
  BracketGroupsPublicClient,
  BracketGroupsUserClient,
} from "@march-madness/client";
import type { GroupData, MemberData } from "@march-madness/client";
import { type Hex, keccak256, toHex } from "viem";

import { GROUPS_CONTRACT_ADDRESS } from "../lib/constants";

/** Hash a human-readable passphrase into sbytes12: take first 12 bytes of keccak256. */
export function passphraseToBytes12(passphrase: string): Hex {
  const hash = keccak256(toHex(passphrase));
  // keccak256 returns 0x + 64 hex chars (32 bytes). Take first 12 bytes = 24 hex chars.
  return `0x${hash.slice(2, 26)}` as Hex;
}

const STORAGE_KEY = "mm-groups";

/**
 * Per-group localStorage entry.
 * - `admin` (optional): true if user created this group
 * - `passphrase` (optional): human-readable passphrase for private groups (what users share)
 * - `password` (optional): hex bytes12 derived from passphrase (what the contract sees)
 */
export interface StoredGroupInfo {
  admin?: true;
  passphrase?: string;
  password?: Hex;
}

/** Full localStorage dict: groupId → StoredGroupInfo */
type StoredGroups = Record<string, StoredGroupInfo>;

function loadStoredGroups(): StoredGroups {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

function saveStoredGroups(groups: StoredGroups) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(groups));
}

export interface JoinedGroup {
  groupId: number;
  group: GroupData;
  members: MemberData[];
  storedInfo: StoredGroupInfo;
}

const isZeroAddress = (addr: string) =>
  !addr || addr === "0x0000000000000000000000000000000000000000";

/**
 * Hook for interacting with BracketGroups contract.
 * Tracks joined groups in localStorage as a JSON dict with optional admin/password metadata.
 */
export function useGroups() {
  const { walletClient, publicClient } = useShieldedWallet();
  const [storedGroups, setStoredGroups] = useState<StoredGroups>(loadStoredGroups);
  const [joinedGroups, setJoinedGroups] = useState<JoinedGroup[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const hasContract = !isZeroAddress(GROUPS_CONTRACT_ADDRESS);
  const joinedGroupIds = useMemo(() => Object.keys(storedGroups).map(Number), [storedGroups]);

  const groupsPublic = useMemo(() => {
    if (!publicClient || !hasContract) return null;
    return new BracketGroupsPublicClient(publicClient, GROUPS_CONTRACT_ADDRESS);
  }, [publicClient, hasContract]);

  const groupsUser = useMemo(() => {
    if (!publicClient || !walletClient || !hasContract) return null;
    return new BracketGroupsUserClient(
      publicClient,
      walletClient,
      GROUPS_CONTRACT_ADDRESS,
    );
  }, [publicClient, walletClient, hasContract]);

  // Refresh group data for all joined groups
  const refreshGroups = useCallback(async () => {
    if (!groupsPublic || joinedGroupIds.length === 0) {
      setJoinedGroups([]);
      return;
    }

    try {
      const results = await Promise.all(
        joinedGroupIds.map(async (groupId) => {
          try {
            const group = await groupsPublic.getGroup(groupId);
            const members = await groupsPublic.getMembers(groupId);
            const storedInfo = storedGroups[String(groupId)] ?? {};
            return { groupId, group, members, storedInfo };
          } catch {
            return null;
          }
        }),
      );
      setJoinedGroups(results.filter((r): r is JoinedGroup => r !== null));
    } catch {
      // ignore refresh errors
    }
  }, [groupsPublic, joinedGroupIds, storedGroups]);

  useEffect(() => {
    refreshGroups();
  }, [refreshGroups]);

  /** Track a group in localStorage. */
  const trackGroup = useCallback(
    (groupId: number, info: StoredGroupInfo = {}) => {
      const updated = { ...storedGroups, [String(groupId)]: { ...storedGroups[String(groupId)], ...info } };
      setStoredGroups(updated);
      saveStoredGroups(updated);
    },
    [storedGroups],
  );

  /** Remove a group from localStorage. */
  const untrackGroup = useCallback(
    (groupId: number) => {
      const updated = { ...storedGroups };
      delete updated[String(groupId)];
      setStoredGroups(updated);
      saveStoredGroups(updated);
    },
    [storedGroups],
  );

  /** Join a public group. */
  const joinGroup = useCallback(
    async (groupId: number, name: string, entryFee: bigint = 0n) => {
      if (!groupsUser) throw new Error("Wallet not connected");
      setIsLoading(true);
      setError(null);
      try {
        const hash = await groupsUser.joinGroup(groupId, name, entryFee);
        trackGroup(groupId);
        await refreshGroups();
        return hash;
      } catch (err) {
        const msg = err instanceof Error ? err.message : "Failed to join group";
        setError(msg);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [groupsUser, trackGroup, refreshGroups],
  );

  /** Join a password-protected group. Takes a human-readable passphrase. */
  const joinGroupWithPassword = useCallback(
    async (
      groupId: number,
      passphrase: string,
      name: string,
      entryFee: bigint = 0n,
    ) => {
      if (!groupsUser) throw new Error("Wallet not connected");
      setIsLoading(true);
      setError(null);
      try {
        const password = passphraseToBytes12(passphrase);
        const hash = await groupsUser.joinGroupWithPassword(
          groupId,
          password,
          name,
          entryFee,
        );
        trackGroup(groupId, { passphrase, password });
        await refreshGroups();
        return hash;
      } catch (err) {
        const msg = err instanceof Error ? err.message : "Failed to join group";
        setError(msg);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [groupsUser, trackGroup, refreshGroups],
  );

  /** Create a public group. Tracks as admin. */
  const createGroup = useCallback(
    async (slug: string, displayName: string, entryFee: bigint) => {
      if (!groupsUser) throw new Error("Wallet not connected");
      setIsLoading(true);
      setError(null);
      try {
        const hash = await groupsUser.createGroup(slug, displayName, entryFee);
        // TODO: parse groupId from event logs once available
        await refreshGroups();
        return hash;
      } catch (err) {
        const msg = err instanceof Error ? err.message : "Failed to create group";
        setError(msg);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [groupsUser, refreshGroups],
  );

  /** Create a password-protected group. Takes a human-readable passphrase. */
  const createGroupWithPassword = useCallback(
    async (slug: string, displayName: string, entryFee: bigint, passphrase: string) => {
      if (!groupsUser) throw new Error("Wallet not connected");
      setIsLoading(true);
      setError(null);
      try {
        const password = passphraseToBytes12(passphrase);
        const hash = await groupsUser.createGroupWithPassword(slug, displayName, entryFee, password);
        // TODO: parse groupId from event logs to trackGroup(groupId, { admin: true, password })
        await refreshGroups();
        return { hash, password };
      } catch (err) {
        const msg = err instanceof Error ? err.message : "Failed to create group";
        setError(msg);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [groupsUser, refreshGroups],
  );

  /** Leave a group. */
  const leaveGroup = useCallback(
    async (groupId: number) => {
      if (!groupsUser) throw new Error("Wallet not connected");
      setIsLoading(true);
      setError(null);
      try {
        const hash = await groupsUser.leaveGroup(groupId);
        untrackGroup(groupId);
        await refreshGroups();
        return hash;
      } catch (err) {
        const msg = err instanceof Error ? err.message : "Failed to leave group";
        setError(msg);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [groupsUser, untrackGroup, refreshGroups],
  );

  /** Edit display name in a group. */
  const editEntryName = useCallback(
    async (groupId: number, name: string) => {
      if (!groupsUser) throw new Error("Wallet not connected");
      setIsLoading(true);
      setError(null);
      try {
        const hash = await groupsUser.editEntryName(groupId, name);
        await refreshGroups();
        return hash;
      } catch (err) {
        const msg = err instanceof Error ? err.message : "Failed to edit name";
        setError(msg);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [groupsUser, refreshGroups],
  );

  /** Look up a group by slug. Returns [groupId, groupData] or null. */
  const lookupGroupBySlug = useCallback(
    async (slug: string) => {
      if (!groupsPublic) return null;
      try {
        return await groupsPublic.getGroupBySlug(slug);
      } catch {
        return null;
      }
    },
    [groupsPublic],
  );

  return {
    hasContract,
    joinedGroups,
    joinedGroupIds,
    isLoading,
    error,
    joinGroup,
    joinGroupWithPassword,
    createGroup,
    createGroupWithPassword,
    leaveGroup,
    editEntryName,
    trackGroup,
    untrackGroup,
    lookupGroupBySlug,
    refreshGroups,
  };
}

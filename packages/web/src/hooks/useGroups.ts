import { useCallback, useEffect, useMemo, useState } from "react";
import { useShieldedWallet } from "seismic-react";
import {
  BracketGroupsPublicClient,
  BracketGroupsUserClient,
} from "@march-madness/client";
import type { GroupData, MemberData } from "@march-madness/client";

import { GROUPS_CONTRACT_ADDRESS } from "../lib/constants";

const STORAGE_KEY = "mm-joined-groups";

/** Group IDs the user has joined, persisted in localStorage. */
function loadJoinedGroupIds(): number[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

function saveJoinedGroupIds(ids: number[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(ids));
}

export interface JoinedGroup {
  groupId: number;
  group: GroupData;
  members: MemberData[];
}

const isZeroAddress = (addr: string) =>
  !addr || addr === "0x0000000000000000000000000000000000000000";

/**
 * Hook for interacting with BracketGroups contract.
 * Tracks joined groups in localStorage and provides group lifecycle methods.
 */
export function useGroups() {
  const { walletClient, publicClient } = useShieldedWallet();
  const [joinedGroupIds, setJoinedGroupIds] = useState<number[]>(loadJoinedGroupIds);
  const [joinedGroups, setJoinedGroups] = useState<JoinedGroup[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const hasContract = !isZeroAddress(GROUPS_CONTRACT_ADDRESS);

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
            return { groupId, group, members };
          } catch {
            return null;
          }
        }),
      );
      setJoinedGroups(results.filter((r): r is JoinedGroup => r !== null));
    } catch {
      // ignore refresh errors
    }
  }, [groupsPublic, joinedGroupIds]);

  useEffect(() => {
    refreshGroups();
  }, [refreshGroups]);

  /** Track a group ID in localStorage after joining. */
  const trackGroup = useCallback(
    (groupId: number) => {
      const updated = [...new Set([...joinedGroupIds, groupId])];
      setJoinedGroupIds(updated);
      saveJoinedGroupIds(updated);
    },
    [joinedGroupIds],
  );

  /** Remove a group ID from localStorage after leaving. */
  const untrackGroup = useCallback(
    (groupId: number) => {
      const updated = joinedGroupIds.filter((id) => id !== groupId);
      setJoinedGroupIds(updated);
      saveJoinedGroupIds(updated);
    },
    [joinedGroupIds],
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

  /** Join a password-protected group. */
  const joinGroupWithPassword = useCallback(
    async (
      groupId: number,
      password: `0x${string}`,
      name: string,
      entryFee: bigint = 0n,
    ) => {
      if (!groupsUser) throw new Error("Wallet not connected");
      setIsLoading(true);
      setError(null);
      try {
        const hash = await groupsUser.joinGroupWithPassword(
          groupId,
          password,
          name,
          entryFee,
        );
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
    leaveGroup,
    editEntryName,
    trackGroup,
    lookupGroupBySlug,
    refreshGroups,
  };
}

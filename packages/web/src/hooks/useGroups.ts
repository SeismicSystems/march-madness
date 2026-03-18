import { useCallback, useEffect, useMemo, useState } from "react";
import { useShieldedWallet } from "seismic-react";
import {
  BracketGroupsPublicClient,
  BracketGroupsUserClient,
} from "@march-madness/client";
import type { GroupData, MemberData } from "@march-madness/client";
import { type Hex, keccak256, toHex } from "viem";

import { API_BASE } from "../lib/api";
import { GROUPS_CONTRACT_ADDRESS } from "../lib/constants";

/** Hash a human-readable passphrase into sbytes12: take first 12 bytes of keccak256. */
export function passphraseToBytes12(passphrase: string): Hex {
  const hash = keccak256(toHex(passphrase));
  // keccak256 returns 0x + 64 hex chars (32 bytes). Take first 12 bytes = 24 hex chars.
  return `0x${hash.slice(2, 26)}` as Hex;
}

// ── Passphrase-only localStorage (client-side secrets) ──────────────

const PASSPHRASE_KEY = "mm-group-passphrases";

interface StoredPassphrase {
  passphrase: string;
  password: Hex;
}

type PassphraseStore = Record<string, StoredPassphrase>;

function loadPassphrases(): PassphraseStore {
  try {
    const raw = localStorage.getItem(PASSPHRASE_KEY);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

function savePassphrases(store: PassphraseStore) {
  localStorage.setItem(PASSPHRASE_KEY, JSON.stringify(store));
}

// Migrate old "mm-groups" localStorage to new passphrase-only store.
// This runs once and cleans up the old key.
function migrateOldStorage(): void {
  try {
    const old = localStorage.getItem("mm-groups");
    if (!old) return;
    const parsed = JSON.parse(old) as Record<string, { passphrase?: string; password?: Hex }>;
    const existing = loadPassphrases();
    let migrated = false;
    for (const [id, info] of Object.entries(parsed)) {
      if (info.passphrase && info.password && !existing[id]) {
        existing[id] = { passphrase: info.passphrase, password: info.password };
        migrated = true;
      }
    }
    if (migrated) savePassphrases(existing);
    localStorage.removeItem("mm-groups");
  } catch {
    // Best-effort migration; don't block on errors.
    localStorage.removeItem("mm-groups");
  }
}

// Run migration on module load.
migrateOldStorage();

// ── Types ────────────────────────────────────────────────────────────

/**
 * Per-group metadata for components. `admin` is derived from on-chain
 * creator address; `passphrase`/`password` come from local storage.
 */
export interface StoredGroupInfo {
  admin?: true;
  passphrase?: string;
  password?: Hex;
}

export interface JoinedGroup {
  groupId: number;
  group: GroupData;
  members: MemberData[];
  storedInfo: StoredGroupInfo;
}

const isZeroAddress = (addr: string) =>
  !addr || addr === "0x0000000000000000000000000000000000000000";

// ── API fetch ────────────────────────────────────────────────────────

interface ApiGroupResponse {
  id: string;
  slug: string;
  display_name: string;
  creator: string;
  has_password: boolean;
  member_count: number;
}

/** Fetch group IDs for an address from the server API. */
async function fetchMyGroupIds(address: string): Promise<number[]> {
  const res = await fetch(`${API_BASE}/address/${address.toLowerCase()}/groups`);
  if (!res.ok) return [];
  const groups: ApiGroupResponse[] = await res.json();
  return groups.map((g) => Number(g.id));
}

// ── Hook ─────────────────────────────────────────────────────────────

/**
 * Hook for interacting with BracketGroups contract.
 * Group membership is sourced from the server API (Redis-backed).
 * Only passphrases (client-side secrets) are stored in localStorage.
 */
export function useGroups() {
  const { walletClient, publicClient } = useShieldedWallet();
  const [joinedGroupIds, setJoinedGroupIds] = useState<number[]>([]);
  const [joinedGroups, setJoinedGroups] = useState<JoinedGroup[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [passphrases, setPassphrases] = useState<PassphraseStore>(loadPassphrases);

  const hasContract = !isZeroAddress(GROUPS_CONTRACT_ADDRESS);
  const walletAddress = walletClient?.account?.address?.toLowerCase();

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

  /** Save a passphrase to localStorage. */
  const storePassphrase = useCallback(
    (groupId: number, passphrase: string, password: Hex) => {
      const updated = { ...passphrases, [String(groupId)]: { passphrase, password } };
      setPassphrases(updated);
      savePassphrases(updated);
    },
    [passphrases],
  );

  /** Hydrate group IDs with on-chain data + local passphrases. */
  const hydrateGroups = useCallback(
    async (ids: number[]) => {
      if (!groupsPublic || ids.length === 0) {
        setJoinedGroups([]);
        return;
      }

      const results = await Promise.all(
        ids.map(async (groupId) => {
          try {
            const group = await groupsPublic.getGroup(groupId);
            const members = await groupsPublic.getMembers(groupId);
            const pp = passphrases[String(groupId)];
            const storedInfo: StoredGroupInfo = {
              ...(walletAddress && group.creator.toLowerCase() === walletAddress
                ? { admin: true as const }
                : {}),
              ...(pp ? { passphrase: pp.passphrase, password: pp.password } : {}),
            };
            return { groupId, group, members, storedInfo };
          } catch {
            return null;
          }
        }),
      );
      setJoinedGroups(results.filter((r): r is JoinedGroup => r !== null));
    },
    [groupsPublic, passphrases, walletAddress],
  );

  /** Fetch group membership from API and hydrate with on-chain data. */
  const refreshGroups = useCallback(async () => {
    if (!walletAddress || !groupsPublic) {
      setJoinedGroupIds([]);
      setJoinedGroups([]);
      return;
    }

    try {
      const ids = await fetchMyGroupIds(walletAddress);
      setJoinedGroupIds(ids);
      await hydrateGroups(ids);
    } catch {
      // ignore refresh errors
    }
  }, [walletAddress, groupsPublic, hydrateGroups]);

  // Fetch on wallet connect / address change.
  useEffect(() => {
    refreshGroups();
  }, [refreshGroups]);

  /** Join a public group. */
  const joinGroup = useCallback(
    async (groupId: number, name: string, entryFee: bigint = 0n) => {
      if (!groupsUser) throw new Error("Wallet not connected");
      setIsLoading(true);
      setError(null);
      try {
        const hash = await groupsUser.joinGroup(groupId, name, entryFee);
        // Optimistic: add ID to local state, then refresh from API.
        setJoinedGroupIds((prev) =>
          prev.includes(groupId) ? prev : [...prev, groupId],
        );
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
    [groupsUser, refreshGroups],
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
        storePassphrase(groupId, passphrase, password);
        setJoinedGroupIds((prev) =>
          prev.includes(groupId) ? prev : [...prev, groupId],
        );
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
    [groupsUser, storePassphrase, refreshGroups],
  );

  /** Create a public group. Creator is auto-joined. */
  const createGroup = useCallback(
    async (slug: string, displayName: string, entryFee: bigint) => {
      if (!groupsUser || !groupsPublic) throw new Error("Wallet not connected");
      setIsLoading(true);
      setError(null);
      try {
        const hash = await groupsUser.createGroup(slug, displayName, entryFee);
        // Look up group by slug to get the ID.
        const result = await groupsPublic.getGroupBySlug(slug);
        setJoinedGroupIds((prev) =>
          prev.includes(result[0]) ? prev : [...prev, result[0]],
        );
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
    [groupsUser, groupsPublic, refreshGroups],
  );

  /** Create a password-protected group. Takes a human-readable passphrase. Creator is auto-joined. */
  const createGroupWithPassword = useCallback(
    async (slug: string, displayName: string, entryFee: bigint, passphrase: string) => {
      if (!groupsUser || !groupsPublic) throw new Error("Wallet not connected");
      setIsLoading(true);
      setError(null);
      try {
        const password = passphraseToBytes12(passphrase);
        const hash = await groupsUser.createGroupWithPassword(slug, displayName, entryFee, password);
        const result = await groupsPublic.getGroupBySlug(slug);
        storePassphrase(result[0], passphrase, password);
        setJoinedGroupIds((prev) =>
          prev.includes(result[0]) ? prev : [...prev, result[0]],
        );
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
    [groupsUser, groupsPublic, storePassphrase, refreshGroups],
  );

  /** Leave a group. */
  const leaveGroup = useCallback(
    async (groupId: number) => {
      if (!groupsUser) throw new Error("Wallet not connected");
      setIsLoading(true);
      setError(null);
      try {
        const hash = await groupsUser.leaveGroup(groupId);
        // Optimistic: remove from local state.
        setJoinedGroupIds((prev) => prev.filter((id) => id !== groupId));
        setJoinedGroups((prev) => prev.filter((g) => g.groupId !== groupId));
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
    [groupsUser, refreshGroups],
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
    async (slug: string): Promise<[number, GroupData] | null> => {
      if (!groupsPublic) return null;
      try {
        return await groupsPublic.getGroupBySlug(slug);
      } catch {
        return null;
      }
    },
    [groupsPublic],
  );

  /** Look up a group by ID. Returns GroupData or null. */
  const lookupGroupById = useCallback(
    async (groupId: number): Promise<GroupData | null> => {
      if (!groupsPublic) return null;
      try {
        const group = await groupsPublic.getGroup(groupId);
        if (!group.creator || group.creator === "0x0000000000000000000000000000000000000000") {
          return null;
        }
        return group;
      } catch {
        return null;
      }
    },
    [groupsPublic],
  );

  // Keep trackGroup/untrackGroup as no-ops for backward compat.
  // Components that called these (e.g. PublicGroupsList after joinGroup)
  // no longer need them since membership comes from the API.
  const trackGroup = useCallback((_groupId: number, _info?: StoredGroupInfo) => {}, []);
  const untrackGroup = useCallback((_groupId: number) => {}, []);

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
    lookupGroupById,
    refreshGroups,
  };
}

export type UseGroupsReturn = ReturnType<typeof useGroups>;

import { useCallback, useEffect, useMemo, useState } from "react";
import { usePrivy } from "@privy-io/react-auth";
import { useShieldedWallet } from "seismic-react";
import {
  BracketGroupsPublicClient,
  BracketGroupsUserClient,
  MarchMadnessPublicClient,
} from "@march-madness/client";
import type {
  GroupData,
  GroupPayoutData,
  MemberData,
} from "@march-madness/client";

import { CONTRACT_ADDRESS, GROUPS_CONTRACT_ADDRESS } from "../lib/constants";

const SCORING_DURATION = 7n * 24n * 3600n; // 7 days in seconds

function nowSeconds(): bigint {
  return BigInt(Math.floor(Date.now() / 1000));
}

export interface GroupWinningsState {
  groupId: number | null;
  groupData: GroupData | null;
  members: MemberData[] | null;
  payouts: GroupPayoutData | null;
  allScored: boolean;
  payoutAmount: bigint | null;
  resultsPostedAt: bigint | null;
  memberIndex: number | null;
  isWinner: boolean;
  hasCollected: boolean;
  canClaim: boolean;
  collectWinnings: () => Promise<`0x${string}`>;
  scoreAllMembers: () => Promise<void>;
  isScoringAll: boolean;
  isCollecting: boolean;
  error: string | null;
  isLoading: boolean;
}

export function useGroupWinningsState(
  slug: string | undefined
): GroupWinningsState {
  const { authenticated } = usePrivy();
  const { walletClient, publicClient } = useShieldedWallet();

  // ── State ────────────────────────────────────────────────────────
  const [groupId, setGroupId] = useState<number | null>(null);
  const [groupData, setGroupData] = useState<GroupData | null>(null);
  const [members, setMembers] = useState<MemberData[] | null>(null);
  const [payouts, setPayouts] = useState<GroupPayoutData | null>(null);
  const [hasCollected, setHasCollected] = useState(false);
  const [resultsPostedAt, setResultsPostedAt] = useState<bigint | null>(null);

  const [isScoringAll, setIsScoringAll] = useState(false);
  const [isCollecting, setIsCollecting] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const walletAddress = walletClient?.account?.address ?? null;
  const walletKey = walletAddress?.toLowerCase() ?? null;

  const isZeroAddress = (addr: string) =>
    !addr || addr === "0x0000000000000000000000000000000000000000";

  const groupsPublic = useMemo(() => {
    if (!publicClient || isZeroAddress(GROUPS_CONTRACT_ADDRESS)) return null;
    return new BracketGroupsPublicClient(publicClient, GROUPS_CONTRACT_ADDRESS);
  }, [publicClient]);

  const groupsUser = useMemo(() => {
    if (!authenticated || !publicClient || !walletClient || !walletAddress)
      return null;
    if (isZeroAddress(GROUPS_CONTRACT_ADDRESS)) return null;
    return new BracketGroupsUserClient(
      publicClient,
      walletClient,
      GROUPS_CONTRACT_ADDRESS
    );
  }, [authenticated, publicClient, walletClient, walletAddress]);

  const mmPublic = useMemo(() => {
    if (!publicClient) return null;
    return new MarchMadnessPublicClient(publicClient, CONTRACT_ADDRESS);
  }, [publicClient]);

  // ── Data fetch ───────────────────────────────────────────────────

  const fetchState = useCallback(async () => {
    if (!groupsPublic || !mmPublic || !slug) return;
    setIsLoading(true);
    try {
      // Step 1: resolve slug → groupId + groupData
      const [resolvedGroupId, resolvedGroupData] =
        await groupsPublic.getGroupBySlug(slug);

      // Step 2: parallel reads
      const [resolvedMembers, resolvedPayouts, rpa, hcw] = await Promise.all([
        groupsPublic.getMembers(resolvedGroupId),
        groupsPublic.getPayouts(resolvedGroupId),
        mmPublic.getResultsPostedAt(),
        walletAddress
          ? groupsPublic.getHasCollectedWinnings(resolvedGroupId, walletAddress)
          : Promise.resolve(false),
      ]);

      setGroupId(resolvedGroupId);
      setGroupData(resolvedGroupData);
      setMembers(resolvedMembers);
      setPayouts(resolvedPayouts);
      setResultsPostedAt(rpa);
      setHasCollected(hcw);
    } catch {
      // Silently ignore poll errors
    } finally {
      setIsLoading(false);
    }
  }, [groupsPublic, mmPublic, slug, walletAddress]);

  useEffect(() => {
    fetchState();
    const id = setInterval(fetchState, 30_000);
    return () => clearInterval(id);
  }, [fetchState]);

  // ── Derived values ───────────────────────────────────────────────

  const scoringWindowClosesAt =
    resultsPostedAt !== null && resultsPostedAt > 0n
      ? resultsPostedAt + SCORING_DURATION
      : null;

  const now = nowSeconds();
  const isWindowClosed =
    resultsPostedAt !== null &&
    resultsPostedAt > 0n &&
    now >= (scoringWindowClosesAt ?? 0n);

  const allScored =
    members !== null && members.length > 0 && members.every((m) => m.isScored);

  const memberIndex = useMemo(() => {
    if (!members || !walletKey) return null;
    const idx = members.findIndex((m) => m.addr.toLowerCase() === walletKey);
    return idx >= 0 ? idx : null;
  }, [members, walletKey]);

  const isWinner =
    memberIndex !== null &&
    members !== null &&
    payouts !== null &&
    payouts.numWinners > 0 &&
    members[memberIndex].isScored &&
    members[memberIndex].score === payouts.winningScore;

  const canClaim = isWinner && isWindowClosed && !hasCollected;

  const payoutAmount =
    groupData !== null &&
    groupData.entryCount > 0 &&
    payouts !== null &&
    payouts.numWinners > 0
      ? (BigInt(groupData.entryCount) * groupData.entryFee) /
        BigInt(payouts.numWinners)
      : null;

  // ── Actions ──────────────────────────────────────────────────────

  const collectWinnings = useCallback(async (): Promise<`0x${string}`> => {
    if (!groupsUser) throw new Error("Wallet not connected");
    if (groupId === null) throw new Error("Group not loaded");
    setIsCollecting(true);
    setError(null);
    try {
      const hash = await groupsUser.collectWinnings(groupId);
      setHasCollected(true);
      return hash;
    } catch (err) {
      const msg =
        err instanceof Error ? err.message : "Failed to collect winnings";
      setError(msg);
      throw err;
    } finally {
      setIsCollecting(false);
    }
  }, [groupsUser, groupId]);

  const scoreAllMembers = useCallback(async (): Promise<void> => {
    if (!groupsUser || !groupsPublic || groupId === null || members === null)
      return;
    setIsScoringAll(true);
    setError(null);
    try {
      for (let i = 0; i < members.length; i++) {
        if (!members[i].isScored) {
          await groupsUser.scoreEntry(groupId, i);
        }
      }
      // Re-fetch members after scoring
      const [updatedMembers, updatedPayouts] = await Promise.all([
        groupsPublic.getMembers(groupId),
        groupsPublic.getPayouts(groupId),
      ]);
      setMembers(updatedMembers);
      setPayouts(updatedPayouts);
    } catch (err) {
      const msg =
        err instanceof Error ? err.message : "Failed to score members";
      setError(msg);
    } finally {
      setIsScoringAll(false);
    }
  }, [groupsUser, groupsPublic, groupId, members]);

  return {
    groupId,
    groupData,
    members,
    payouts,
    allScored,
    payoutAmount,
    resultsPostedAt,
    memberIndex,
    isWinner,
    hasCollected,
    canClaim,
    collectWinnings,
    scoreAllMembers,
    isScoringAll,
    isCollecting,
    error,
    isLoading,
  };
}

import { useCallback, useEffect, useMemo, useState } from "react";
import { usePrivy } from "@privy-io/react-auth";
import { useShieldedWallet } from "seismic-react";
import {
  MarchMadnessPublicClient,
  MarchMadnessUserClient,
} from "@march-madness/client";

import { CONTRACT_ADDRESS } from "../lib/constants";

// Contract constants — both are unchanging
const SCORING_DURATION = 7n * 24n * 3600n; // 7 days in seconds
const RESULTS_DEADLINE = 90n * 24n * 3600n; // 90 days in seconds

function nowSeconds(): bigint {
  return BigInt(Math.floor(Date.now() / 1000));
}

export interface WinningsState {
  resultsPostedAt: bigint | null;
  isWindowOpen: boolean;
  isWindowClosed: boolean;
  scoringWindowClosesAt: bigint | null;
  winningScore: number | null;
  numWinners: bigint | null;
  payoutAmount: bigint | null;
  walletScore: number | null;
  walletIsScored: boolean;
  hasCollected: boolean;
  isWinner: boolean;
  canClaim: boolean;
  canClaimEntryFee: boolean;
  collectWinnings: () => Promise<`0x${string}`>;
  collectEntryFee: () => Promise<`0x${string}`>;
  isCollecting: boolean;
  error: string | null;
  isLoading: boolean;
}

export function useWinningsState(): WinningsState {
  const { authenticated } = usePrivy();
  const { walletClient, publicClient } = useShieldedWallet();

  // ── Contract state ───────────────────────────────────────────────
  const [resultsPostedAt, setResultsPostedAt] = useState<bigint | null>(null);
  const [winningScore, setWinningScore] = useState<number | null>(null);
  const [numWinners, setNumWinners] = useState<bigint | null>(null);
  const [numEntries, setNumEntries] = useState<number | null>(null);
  const [entryFee, setEntryFee] = useState<bigint | null>(null);
  const [submissionDeadline, setSubmissionDeadline] = useState<bigint | null>(
    null
  );
  // Wallet-scoped state — keyed by address to avoid stale reads
  const [walletData, setWalletData] = useState<{
    owner: string;
    score: number;
    isScored: boolean;
    hasCollectedWinnings: boolean;
    hasCollectedEntryFee: boolean;
    hasEntry: boolean;
  } | null>(null);

  const [isCollecting, setIsCollecting] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const walletAddress = walletClient?.account?.address ?? null;
  const walletKey = walletAddress?.toLowerCase() ?? null;

  const mmPublic = useMemo(() => {
    if (!publicClient) return null;
    return new MarchMadnessPublicClient(publicClient, CONTRACT_ADDRESS);
  }, [publicClient]);

  const mmUser = useMemo(() => {
    if (!authenticated || !publicClient || !walletClient || !walletAddress)
      return null;
    return new MarchMadnessUserClient(
      publicClient,
      walletClient,
      CONTRACT_ADDRESS
    );
  }, [authenticated, publicClient, walletClient, walletAddress]);

  // ── Data fetch ───────────────────────────────────────────────────

  const fetchState = useCallback(async () => {
    if (!mmPublic) return;
    setIsLoading(true);
    try {
      const [rpa, ws, nw, ne, ef, sd] = await Promise.all([
        mmPublic.getResultsPostedAt(),
        mmPublic.getWinningScore(),
        mmPublic.getNumWinners(),
        mmPublic.getEntryCount(),
        mmPublic.getEntryFee(),
        mmPublic.getSubmissionDeadline(),
      ]);
      setResultsPostedAt(rpa);
      setWinningScore(ws);
      setNumWinners(nw);
      setNumEntries(ne);
      setEntryFee(ef);
      setSubmissionDeadline(sd);

      // Wallet-specific reads
      if (walletAddress) {
        const [score, isScored, hcw, hcef, hasEntry] = await Promise.all([
          mmPublic.getScore(walletAddress),
          mmPublic.getIsScored(walletAddress),
          mmPublic.getHasCollectedWinnings(walletAddress),
          mmPublic.getHasCollectedEntryFee(walletAddress),
          mmPublic.getHasEntry(walletAddress),
        ]);
        setWalletData({
          owner: walletAddress.toLowerCase(),
          score,
          isScored,
          hasCollectedWinnings: hcw,
          hasCollectedEntryFee: hcef,
          hasEntry,
        });
      } else {
        setWalletData(null);
      }
    } catch (err) {
      // Silently ignore poll errors — stale data is fine
    } finally {
      setIsLoading(false);
    }
  }, [mmPublic, walletAddress]);

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
  const isWindowOpen =
    resultsPostedAt !== null &&
    resultsPostedAt > 0n &&
    now < (scoringWindowClosesAt ?? 0n);
  const isWindowClosed =
    resultsPostedAt !== null &&
    resultsPostedAt > 0n &&
    now >= (scoringWindowClosesAt ?? 0n);

  const currentWalletData =
    walletData && walletKey && walletData.owner === walletKey
      ? walletData
      : null;

  const walletScore = currentWalletData?.score ?? null;
  const walletIsScored = currentWalletData?.isScored ?? false;
  const hasCollected = currentWalletData?.hasCollectedWinnings ?? false;
  const hasCollectedEntryFee = currentWalletData?.hasCollectedEntryFee ?? false;
  const hasEntry = currentWalletData?.hasEntry ?? false;

  const isWinner =
    walletIsScored &&
    walletScore !== null &&
    winningScore !== null &&
    walletScore === winningScore &&
    numWinners !== null &&
    numWinners > 0n;

  const canClaim = isWinner && isWindowClosed && !hasCollected;

  const noContestAt =
    submissionDeadline !== null ? submissionDeadline + RESULTS_DEADLINE : null;

  const canClaimEntryFee =
    resultsPostedAt !== null &&
    resultsPostedAt === 0n &&
    noContestAt !== null &&
    now > noContestAt &&
    hasEntry &&
    !hasCollectedEntryFee;

  const payoutAmount =
    numWinners !== null &&
    numWinners > 0n &&
    numEntries !== null &&
    entryFee !== null
      ? (BigInt(numEntries) * entryFee) / numWinners
      : null;

  // ── Actions ──────────────────────────────────────────────────────

  const collectWinnings = useCallback(async (): Promise<`0x${string}`> => {
    if (!mmUser) throw new Error("Wallet not connected");
    setIsCollecting(true);
    setError(null);
    try {
      const hash = await mmUser.collectWinnings();
      // Optimistically mark as collected
      if (walletKey && walletData && walletData.owner === walletKey) {
        setWalletData({ ...walletData, hasCollectedWinnings: true });
      }
      return hash;
    } catch (err) {
      const msg =
        err instanceof Error ? err.message : "Failed to collect winnings";
      setError(msg);
      throw err;
    } finally {
      setIsCollecting(false);
    }
  }, [mmUser, walletKey, walletData]);

  const collectEntryFee = useCallback(async (): Promise<`0x${string}`> => {
    if (!mmUser) throw new Error("Wallet not connected");
    setIsCollecting(true);
    setError(null);
    try {
      const hash = await mmUser.collectEntryFee();
      // Optimistically mark as collected
      if (walletKey && walletData && walletData.owner === walletKey) {
        setWalletData({ ...walletData, hasCollectedEntryFee: true });
      }
      return hash;
    } catch (err) {
      const msg =
        err instanceof Error ? err.message : "Failed to collect entry fee";
      setError(msg);
      throw err;
    } finally {
      setIsCollecting(false);
    }
  }, [mmUser, walletKey, walletData]);

  return {
    resultsPostedAt,
    isWindowOpen,
    isWindowClosed,
    scoringWindowClosesAt,
    winningScore,
    numWinners,
    payoutAmount,
    walletScore,
    walletIsScored,
    hasCollected,
    isWinner,
    canClaim,
    canClaimEntryFee,
    collectWinnings,
    collectEntryFee,
    isCollecting,
    error,
    isLoading,
  };
}

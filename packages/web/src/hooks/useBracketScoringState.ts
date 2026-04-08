import { useCallback, useEffect, useMemo, useState } from "react";
import { usePrivy } from "@privy-io/react-auth";
import { useShieldedWallet } from "seismic-react";
import {
  MarchMadnessPublicClient,
  MarchMadnessUserClient,
} from "@march-madness/client";
import type { Address } from "viem";

import { CONTRACT_ADDRESS } from "../lib/constants";

const SCORING_DURATION = 7n * 24n * 3600n; // 7 days in seconds

function nowSeconds(): bigint {
  return BigInt(Math.floor(Date.now() / 1000));
}

export interface BracketScoringState {
  isScored: boolean;
  canScore: boolean;
  scoreBracket: () => Promise<`0x${string}`>;
  isScoring: boolean;
  error: string | null;
  isLoading: boolean;
}

export function useBracketScoringState(
  targetAddress: string | undefined
): BracketScoringState {
  const { authenticated } = usePrivy();
  const { walletClient, publicClient } = useShieldedWallet();

  const [resultsPostedAt, setResultsPostedAt] = useState<bigint | null>(null);
  const [isScored, setIsScored] = useState(false);
  const [isScoring, setIsScoring] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const walletAddress = walletClient?.account?.address ?? null;

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

  useEffect(() => {
    if (!mmPublic || !targetAddress) return;

    const fetch = async () => {
      setIsLoading(true);
      try {
        const [rpa, scored] = await Promise.all([
          mmPublic.getResultsPostedAt(),
          mmPublic.getIsScored(targetAddress as Address),
        ]);
        setResultsPostedAt(rpa);
        setIsScored(scored);
      } catch {
        // Silently ignore poll errors
      } finally {
        setIsLoading(false);
      }
    };

    fetch();
    const id = setInterval(fetch, 30_000);
    return () => clearInterval(id);
  }, [mmPublic, targetAddress]);

  const scoringWindowClosesAt =
    resultsPostedAt !== null && resultsPostedAt > 0n
      ? resultsPostedAt + SCORING_DURATION
      : null;

  const isWindowOpen =
    resultsPostedAt !== null &&
    resultsPostedAt > 0n &&
    nowSeconds() < (scoringWindowClosesAt ?? 0n);

  const canScore = isWindowOpen && !isScored && mmUser !== null;

  const scoreBracket = useCallback(async (): Promise<`0x${string}`> => {
    if (!mmUser) throw new Error("Wallet not connected");
    if (!targetAddress) throw new Error("No target address");
    setIsScoring(true);
    setError(null);
    try {
      const hash = await mmUser.scoreBracket(targetAddress as Address);
      setIsScored(true);
      return hash;
    } catch (err) {
      const msg =
        err instanceof Error ? err.message : "Failed to score bracket";
      setError(msg);
      throw err;
    } finally {
      setIsScoring(false);
    }
  }, [mmUser, targetAddress]);

  return {
    isScored,
    canScore,
    scoreBracket,
    isScoring,
    error,
    isLoading,
  };
}

import { useCallback, useMemo, useState } from "react";
import { useShieldedWallet } from "seismic-react";
import { formatEther } from "viem";

import {
  MarchMadnessPublicClient,
  MarchMadnessUserClient,
} from "@march-madness/client";
import { usePrivy } from "@privy-io/react-auth";
import { useQuery, useQueryClient } from "@tanstack/react-query";

import { useNow } from "./useNow";
import { usePrivyWalletSelection } from "./usePrivyWalletSelection";
import { debugLog, useDebugValueChanges } from "./useDebugValueChanges";
import { CONTRACT_ADDRESS } from "../lib/constants";
import { normalizeAddress } from "../lib/privyWallets";

const contractQueryKeys = {
  balance: (chainId: number | null, walletKey: string | null) =>
    ["marchMadness", "balance", chainId, walletKey] as const,
  entryCount: (chainId: number | null) =>
    ["marchMadness", "entryCount", chainId] as const,
  entryFee: (chainId: number | null) =>
    ["marchMadness", "entryFee", chainId] as const,
  hasEntry: (chainId: number | null, walletKey: string | null) =>
    ["marchMadness", "hasEntry", chainId, walletKey] as const,
  submissionDeadline: (chainId: number | null) =>
    ["marchMadness", "submissionDeadline", chainId] as const,
};

const stringify = (value: unknown): string => {
  if (typeof value === "string") return value;
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
};

const extractErrorMessage = (err: unknown, fallback: string): string => {
  if (!err) return fallback;
  const parts: string[] = [];
  let current: unknown = err;

  for (let i = 0; i < 8 && current; i++) {
    if (current instanceof Error) {
      const error = current as Error & {
        cause?: unknown;
        details?: unknown;
        shortMessage?: unknown;
      };
      if (error.shortMessage) parts.push(stringify(error.shortMessage));
      else if (error.message) parts.push(error.message);
      if (error.details) parts.push(`details: ${stringify(error.details)}`);
      current = error.cause;
      continue;
    }

    if (typeof current === "object" && current !== null) {
      const value = current as Record<string, unknown>;
      if (value.shortMessage) parts.push(stringify(value.shortMessage));
      else if (value.message) parts.push(stringify(value.message));
      if (value.details) parts.push(`details: ${stringify(value.details)}`);
      if (!value.shortMessage && !value.message && !value.details) {
        parts.push(stringify(value));
      }
      current = value.cause;
      continue;
    }

    parts.push(stringify(current));
    break;
  }

  const deduped = parts.filter((message, index) => {
    return index === 0 || message !== parts[index - 1];
  });
  return deduped.join(" -> ") || fallback;
};

/**
 * Hook for interacting with the MarchMadness contract.
 *
 * On login, checks hasEntry(address) via a public read (no signing).
 * The signed read (getMyBracket) only happens when the user explicitly
 * clicks "Load my bracket".
 */
export function useContract() {
  const { authenticated } = usePrivy();
  const { preferredWalletAddress, privyReady, walletsReady } =
    usePrivyWalletSelection();
  const {
    walletClient,
    publicClient,
    loaded: shieldedLoaded,
    error: shieldedError,
  } = useShieldedWallet();
  const queryClient = useQueryClient();
  const [hasSubmittedState, setHasSubmittedState] = useState<{
    owner: string | null;
    value: boolean;
  }>({ owner: null, value: false });
  const [existingBracketState, setExistingBracketState] = useState<{
    owner: string | null;
    value: `0x${string}` | null;
  }>({ owner: null, value: null });
  const [isLoading, setIsLoading] = useState(false);
  const [isBracketLoading, setIsBracketLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const rawWalletAddress = authenticated
    ? (walletClient?.account?.address ?? null)
    : null;
  const walletAddress =
    preferredWalletAddress &&
    normalizeAddress(rawWalletAddress) === preferredWalletAddress
      ? rawWalletAddress
      : null;
  const walletKey = walletAddress?.toLowerCase() ?? null;
  const existingBracket =
    walletKey && existingBracketState.owner === walletKey
      ? existingBracketState.value
      : null;

  const mmPublic = useMemo(() => {
    if (!publicClient) return null;
    return new MarchMadnessPublicClient(publicClient, CONTRACT_ADDRESS);
  }, [publicClient]);
  const mmUser = useMemo(() => {
    if (!authenticated || !publicClient || !walletClient || !walletAddress) {
      return null;
    }

    return new MarchMadnessUserClient(
      publicClient,
      walletClient,
      CONTRACT_ADDRESS,
    );
  }, [authenticated, publicClient, walletAddress, walletClient]);
  const chainId = publicClient?.chain?.id ?? null;
  const now = useNow();

  const { data: submissionDeadline = null } = useQuery({
    queryKey: contractQueryKeys.submissionDeadline(chainId),
    enabled: !!mmPublic,
    queryFn: async () => Number(await mmPublic!.getSubmissionDeadline()),
    staleTime: Infinity,
  });
  const {
    data: entryCount = 0,
    refetch: refetchEntryCount,
  } = useQuery({
    queryKey: contractQueryKeys.entryCount(chainId),
    enabled: !!mmPublic,
    queryFn: async () => Number(await mmPublic!.getEntryCount()),
    initialData: 0,
  });
  const {
    data: entryFeeDisplay = null,
  } = useQuery({
    queryKey: contractQueryKeys.entryFee(chainId),
    enabled: !!mmPublic,
    queryFn: async () => `${formatEther(await mmPublic!.getEntryFee())} testnet ETH`,
    staleTime: Infinity,
  });
  const hasEntryQuery = useQuery({
    queryKey: contractQueryKeys.hasEntry(chainId, walletKey),
    enabled: !!mmPublic && !!walletAddress && !!walletKey,
    queryFn: async () => await mmPublic!.getHasEntry(walletAddress!),
    retry: false,
  });
  const balanceQuery = useQuery({
    queryKey: contractQueryKeys.balance(chainId, walletKey),
    enabled: !!publicClient && !!walletAddress && !!walletKey,
    queryFn: async () => await publicClient!.getBalance({ address: walletAddress! }),
    retry: false,
  });

  const hasSubmitted =
    walletKey && hasSubmittedState.owner === walletKey
      ? hasSubmittedState.value
      : hasEntryQuery.data ?? false;
  const balance = balanceQuery.data ?? null;
  const hasResolvedEntryState =
    !authenticated || (walletKey !== null && hasEntryQuery.isFetched);
  const hasResolvedBalance =
    !authenticated || (walletKey !== null && balanceQuery.isFetched);
  const isBeforeDeadline =
    submissionDeadline !== null && now / 1000 < submissionDeadline;
  const fetchEntryCount = useCallback(async () => {
    await refetchEntryCount();
  }, [refetchEntryCount]);
  const refetchHasEntry = hasEntryQuery.refetch;
  const refetchBalance = balanceQuery.refetch;

  const isSessionHydrating =
    !privyReady ||
    !walletsReady ||
    (authenticated &&
      ((!preferredWalletAddress || !walletAddress || !shieldedLoaded) ||
        (walletAddress && (!hasResolvedEntryState || !hasResolvedBalance))));

  useDebugValueChanges("useContract", {
    authenticated,
    privyReady,
    walletsReady,
    preferredWalletAddress,
    rawWalletAddress: normalizeAddress(rawWalletAddress),
    walletAddress: normalizeAddress(walletAddress),
    shieldedLoaded,
    shieldedError: shieldedError ? String(shieldedError) : null,
    hasResolvedEntryState,
    hasResolvedBalance,
    hasSubmitted,
    hasExistingBracket: !!existingBracket,
    balance: balance?.toString() ?? null,
    isLoading,
    isBracketLoading,
    isSessionHydrating,
  });

  /**
   * Load user's bracket via signed read (before deadline) or transparent read (after).
   * This is the expensive operation that requires wallet signing — only call on user action.
   */
  const loadMyBracket = useCallback(async () => {
    debugLog("useContract loadMyBracket start", {
      authenticated,
      preferredWalletAddress,
      rawWalletAddress: normalizeAddress(rawWalletAddress),
      walletAddress: normalizeAddress(walletAddress),
      shieldedLoaded,
      hasMmUser: !!mmUser,
    });
    if (!mmUser) throw new Error("Wallet not connected");
    setIsBracketLoading(true);
    setError(null);

    try {
      const bracketHex = await mmUser.getMyBracket();
      if (
        bracketHex &&
        bracketHex !== "0x0000000000000000" &&
        BigInt(bracketHex) !== BigInt(0)
      ) {
        if (walletKey) {
          setExistingBracketState({ owner: walletKey, value: bracketHex });
        }
        debugLog("useContract loadMyBracket success", {
          walletAddress: normalizeAddress(walletAddress),
          hasBracket: true,
        });
        return bracketHex;
      }
      debugLog("useContract loadMyBracket empty", {
        walletAddress: normalizeAddress(walletAddress),
      });
    } catch (err) {
      const message = extractErrorMessage(err, "Failed to load bracket");
      debugLog("useContract loadMyBracket error", {
        walletAddress: normalizeAddress(walletAddress),
        error: message,
      });
      setError(message);
      throw err;
    } finally {
      setIsBracketLoading(false);
    }
    return null;
  }, [
    authenticated,
    mmUser,
    preferredWalletAddress,
    rawWalletAddress,
    shieldedLoaded,
    walletAddress,
    walletKey,
  ]);

  // Submit bracket (shielded write via client library)
  const submitBracket = useCallback(
    async (bracketHex: `0x${string}`) => {
      if (!mmUser) {
        setError(
          "Wallet not connected - please wait for your wallet to initialize or try reconnecting.",
        );
        return;
      }
      setIsLoading(true);
      setError(null);

      try {
        const hash = await mmUser.submitBracket(bracketHex);
        if (walletKey) {
          setHasSubmittedState({ owner: walletKey, value: true });
          setExistingBracketState({ owner: walletKey, value: bracketHex });
          queryClient.setQueryData(
            contractQueryKeys.hasEntry(chainId, walletKey),
            true,
          );
        }
        await Promise.allSettled([
          refetchEntryCount(),
          refetchHasEntry(),
          refetchBalance(),
        ]);
        return hash;
      } catch (err) {
        const message = extractErrorMessage(err, "Failed to submit bracket");
        setError(message);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [
      chainId,
      mmUser,
      queryClient,
      refetchBalance,
      refetchEntryCount,
      refetchHasEntry,
      walletKey,
    ],
  );

  // Update bracket (shielded write, no additional fee)
  const updateBracket = useCallback(
    async (bracketHex: `0x${string}`) => {
      if (!mmUser) {
        setError(
          "Wallet not connected - please wait for your wallet to initialize or try reconnecting.",
        );
        return;
      }
      setIsLoading(true);
      setError(null);

      try {
        const hash = await mmUser.updateBracket(bracketHex);
        if (walletKey) {
          setExistingBracketState({ owner: walletKey, value: bracketHex });
        }
        return hash;
      } catch (err) {
        const message = extractErrorMessage(err, "Failed to update bracket");
        setError(message);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [mmUser, walletKey],
  );

  // Set tag (transparent write via client library)
  const setTag = useCallback(
    async (tag: string) => {
      if (!mmUser) {
        setError(
          "Wallet not connected - please wait for your wallet to initialize or try reconnecting.",
        );
        return;
      }
      setIsLoading(true);
      setError(null);

      try {
        const hash = await mmUser.setTag(tag);
        return hash;
      } catch (err) {
        const message = extractErrorMessage(err, "Failed to set tag");
        setError(message);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [mmUser],
  );

  return {
    entryCount,
    hasSubmitted,
    existingBracket,
    isLoading,
    isBracketLoading,
    error,
    isSessionHydrating,
    isBeforeDeadline,
    submissionDeadline,
    balance,
    entryFeeDisplay,
    submitBracket,
    updateBracket,
    setTag,
    loadMyBracket,
    fetchEntryCount,
    walletAddress,
  };
}

export type UseContractReturn = ReturnType<typeof useContract>;

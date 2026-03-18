import { useCallback, useEffect, useMemo, useState } from "react";
import { useShieldedWallet } from "seismic-react";
import { formatEther } from "viem";

import {
  MarchMadnessPublicClient,
  MarchMadnessUserClient,
} from "@march-madness/client";
import { usePrivy } from "@privy-io/react-auth";

import { usePrivyWalletSelection } from "./usePrivyWalletSelection";
import { debugLog, useDebugValueChanges } from "./useDebugValueChanges";
import { CONTRACT_ADDRESS, SUBMISSION_DEADLINE } from "../lib/constants";
import { normalizeAddress } from "../lib/privyWallets";

/**
 * Fetch the on-chain submission deadline once, falling back to the
 * hardcoded constant if the contract read fails (e.g. no wallet / not deployed).
 */
function useSubmissionDeadline(
  mmPublic: MarchMadnessPublicClient | null,
): number {
  const [deadline, setDeadline] = useState<number>(SUBMISSION_DEADLINE);

  useEffect(() => {
    if (!mmPublic) return;
    let cancelled = false;
    mmPublic
      .getSubmissionDeadline()
      .then((val) => {
        if (!cancelled) setDeadline(Number(val));
      })
      .catch(() => {
        // Contract might not be deployed yet — keep hardcoded fallback
      });
    return () => {
      cancelled = true;
    };
  }, [mmPublic]);

  return deadline;
}

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
  const { walletClient, publicClient, loaded: shieldedLoaded, error: shieldedError } =
    useShieldedWallet();
  const [entryCount, setEntryCount] = useState<number>(0);
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
  const [balanceState, setBalanceState] = useState<{
    owner: string | null;
    value: bigint | null;
  }>({ owner: null, value: null });
  const [entryFeeDisplay, setEntryFeeDisplay] = useState<string | null>(null);
  const [resolvedEntryStateOwner, setResolvedEntryStateOwner] = useState<string | null>(null);
  const [resolvedBalanceOwner, setResolvedBalanceOwner] = useState<string | null>(null);

  // Extract full error detail from nested/wrapped errors (Privy, viem, etc.)
  // Returns all messages in the cause chain so we can debug on mobile.
  // Serializes objects to JSON so we never get "[object Object]".
  const stringify = (v: unknown): string => {
    if (typeof v === "string") return v;
    try { return JSON.stringify(v); } catch { return String(v); }
  };
  const extractErrorMessage = (err: unknown, fallback: string): string => {
    if (!err) return fallback;
    const parts: string[] = [];
    let current: unknown = err;
    for (let i = 0; i < 8 && current; i++) {
      if (current instanceof Error) {
        const e = current as Error & { shortMessage?: unknown; details?: unknown; cause?: unknown };
        if (e.shortMessage) parts.push(stringify(e.shortMessage));
        else if (e.message) parts.push(e.message);
        if (e.details) parts.push(`details: ${stringify(e.details)}`);
        current = e.cause;
      } else if (typeof current === "object" && current !== null) {
        const obj = current as Record<string, unknown>;
        if (obj.shortMessage) parts.push(stringify(obj.shortMessage));
        else if (obj.message) parts.push(stringify(obj.message));
        if (obj.details) parts.push(`details: ${stringify(obj.details)}`);
        if (!obj.shortMessage && !obj.message && !obj.details) {
          parts.push(stringify(obj));
        }
        current = obj.cause;
      } else {
        parts.push(stringify(current));
        break;
      }
    }
    // Deduplicate consecutive identical messages
    const deduped = parts.filter((m, i) => i === 0 || m !== parts[i - 1]);
    return deduped.join(" → ") || fallback;
  };

  const rawWalletAddress = authenticated
    ? (walletClient?.account?.address ?? null)
    : null;
  const walletAddress =
    preferredWalletAddress &&
    normalizeAddress(rawWalletAddress) === preferredWalletAddress
      ? rawWalletAddress
      : null;
  const walletKey = walletAddress?.toLowerCase() ?? null;
  const hasSubmitted =
    walletKey && hasSubmittedState.owner === walletKey
      ? hasSubmittedState.value
      : false;
  const existingBracket =
    walletKey && existingBracketState.owner === walletKey
      ? existingBracketState.value
      : null;
  const balance =
    walletKey && balanceState.owner === walletKey
      ? balanceState.value
      : null;
  const hasResolvedEntryState =
    !authenticated ||
    (walletKey !== null && resolvedEntryStateOwner === walletKey);
  const hasResolvedBalance =
    !authenticated ||
    (walletKey !== null && resolvedBalanceOwner === walletKey);

  // Construct client library instances from seismic-react wallet/public clients
  const mmPublic = useMemo(() => {
    if (!publicClient) return null;
    return new MarchMadnessPublicClient(publicClient, CONTRACT_ADDRESS);
  }, [publicClient]);

  const mmUser = useMemo(() => {
    if (!authenticated || !publicClient || !walletClient || !walletAddress) return null;
    return new MarchMadnessUserClient(
      publicClient,
      walletClient,
      CONTRACT_ADDRESS,
    );
  }, [authenticated, publicClient, walletAddress, walletClient]);

  // On-chain deadline (seconds), with hardcoded fallback
  const submissionDeadline = useSubmissionDeadline(mmPublic);

  // Reactive: recalculate every second so the UI transitions at the right moment
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);
  const isBeforeDeadline = now / 1000 < submissionDeadline;

  // Fetch entry count
  const fetchEntryCount = useCallback(async () => {
    if (!mmPublic) return;
    try {
      const count = await mmPublic.getEntryCount();
      setEntryCount(Number(count));
    } catch {
      // Contract might not be deployed yet
    }
  }, [mmPublic]);

  // Check if user has submitted (public read — no signing needed)
  const checkHasEntry = useCallback(async () => {
    if (!mmPublic || !walletAddress || !walletKey) {
      return;
    }
    try {
      const has = await mmPublic.getHasEntry(walletAddress);
      setHasSubmittedState({ owner: walletKey, value: has });
    } catch {
      // Contract might not be deployed yet
    } finally {
      setResolvedEntryStateOwner(walletKey);
    }
  }, [mmPublic, walletAddress, walletKey]);

  // Fetch entry fee from contract
  const fetchEntryFee = useCallback(async () => {
    if (!mmPublic) return;
    try {
      const fee = await mmPublic.getEntryFee();
      setEntryFeeDisplay(`${formatEther(fee)} testnet ETH`);
    } catch {
      // Contract might not be deployed yet
    }
  }, [mmPublic]);

  // Fetch wallet ETH balance
  const fetchBalance = useCallback(async () => {
    if (!publicClient || !walletAddress || !walletKey) {
      return;
    }
    try {
      const bal = await publicClient.getBalance({ address: walletAddress });
      setBalanceState({ owner: walletKey, value: bal });
    } catch {
      // ignore
    } finally {
      setResolvedBalanceOwner(walletKey);
    }
  }, [publicClient, walletAddress, walletKey]);

  useEffect(() => {
    fetchEntryCount();
    checkHasEntry();
    fetchBalance();
    fetchEntryFee();
  }, [fetchEntryCount, checkHasEntry, fetchBalance, fetchEntryFee]);

  const isSessionHydrating =
    !privyReady ||
    !walletsReady ||
    (authenticated &&
      (((!preferredWalletAddress || !walletAddress || !shieldedLoaded) &&
        !shieldedError) ||
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
      const msg = extractErrorMessage(err, "Failed to load bracket");
      debugLog("useContract loadMyBracket error", {
        walletAddress: normalizeAddress(walletAddress),
        error: msg,
      });
      setError(msg);
      throw err;
    } finally {
      setIsBracketLoading(false);
    }
    return null;
  }, [
    authenticated,
    extractErrorMessage,
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
        setError("Wallet not connected — please wait for your wallet to initialize or try reconnecting.");
        return;
      }
      setIsLoading(true);
      setError(null);

      try {
        const hash = await mmUser.submitBracket(bracketHex);
        if (walletKey) {
          setHasSubmittedState({ owner: walletKey, value: true });
          setExistingBracketState({ owner: walletKey, value: bracketHex });
        }
        await fetchEntryCount();
        await fetchBalance();
        return hash;
      } catch (err) {
        const msg = extractErrorMessage(err, "Failed to submit bracket");
        setError(msg);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [mmUser, fetchEntryCount, fetchBalance, walletKey],
  );

  // Update bracket (shielded write, no additional fee)
  const updateBracket = useCallback(
    async (bracketHex: `0x${string}`) => {
      if (!mmUser) {
        setError("Wallet not connected — please wait for your wallet to initialize or try reconnecting.");
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
        const msg = extractErrorMessage(err, "Failed to update bracket");
        setError(msg);
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
        setError("Wallet not connected — please wait for your wallet to initialize or try reconnecting.");
        return;
      }
      setIsLoading(true);
      setError(null);

      try {
        const hash = await mmUser.setTag(tag);
        return hash;
      } catch (err) {
        const msg = extractErrorMessage(err, "Failed to set tag");
        setError(msg);
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

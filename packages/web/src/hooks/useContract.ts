import { useCallback, useEffect, useState } from "react";
import { useShieldedWallet } from "seismic-react";
import { parseEther } from "viem";

import { marchMadnessAbi } from "../lib/abi";
import { CONTRACT_ADDRESS, SUBMISSION_DEADLINE } from "../lib/constants";

/**
 * Hook for interacting with the MarchMadness contract.
 */
export function useContract() {
  const { walletClient, publicClient } = useShieldedWallet();
  const [entryCount, setEntryCount] = useState<number>(0);
  const [hasSubmitted, setHasSubmitted] = useState(false);
  const [existingBracket, setExistingBracket] = useState<`0x${string}` | null>(
    null,
  );
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const isBeforeDeadline = Date.now() / 1000 < SUBMISSION_DEADLINE;

  // Fetch entry count
  const fetchEntryCount = useCallback(async () => {
    if (!publicClient) return;
    try {
      const count = await publicClient.readContract({
        address: CONTRACT_ADDRESS,
        abi: marchMadnessAbi,
        functionName: "getEntryCount",
      });
      setEntryCount(Number(count));
    } catch {
      // Contract might not be deployed yet
    }
  }, [publicClient]);

  // Fetch user's existing bracket (signed read before deadline)
  const fetchMyBracket = useCallback(async () => {
    if (!walletClient) return;
    const address = walletClient.account?.address;
    if (!address) return;

    try {
      // Before deadline: use signed read (walletClient.readContract)
      // After deadline: use transparent read
      const bracket = isBeforeDeadline
        ? await walletClient.readContract({
            address: CONTRACT_ADDRESS,
            abi: marchMadnessAbi,
            functionName: "getBracket",
            args: [address],
          })
        : await publicClient?.readContract({
            address: CONTRACT_ADDRESS,
            abi: marchMadnessAbi,
            functionName: "getBracket",
            args: [address],
          });

      const bracketHex = bracket as `0x${string}`;
      // Check if bracket has sentinel bit set (meaning it exists)
      if (
        bracketHex &&
        bracketHex !== "0x0000000000000000" &&
        BigInt(bracketHex) !== BigInt(0)
      ) {
        setExistingBracket(bracketHex);
        setHasSubmitted(true);
      }
    } catch {
      // No bracket submitted yet or contract not deployed
    }
  }, [walletClient, publicClient, isBeforeDeadline]);

  useEffect(() => {
    fetchEntryCount();
    fetchMyBracket();
  }, [fetchEntryCount, fetchMyBracket]);

  // Submit bracket (shielded write)
  const submitBracket = useCallback(
    async (bracketHex: `0x${string}`) => {
      if (!walletClient) throw new Error("Wallet not connected");
      setIsLoading(true);
      setError(null);

      try {
        const hash = await walletClient.writeContract({
          address: CONTRACT_ADDRESS,
          abi: marchMadnessAbi,
          functionName: "submitBracket",
          args: [bracketHex],
          value: parseEther("1"),
        });
        setHasSubmitted(true);
        setExistingBracket(bracketHex);
        await fetchEntryCount();
        return hash;
      } catch (err) {
        const msg =
          err instanceof Error ? err.message : "Failed to submit bracket";
        setError(msg);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [walletClient, fetchEntryCount],
  );

  // Update bracket (shielded write, no additional fee)
  const updateBracket = useCallback(
    async (bracketHex: `0x${string}`) => {
      if (!walletClient) throw new Error("Wallet not connected");
      setIsLoading(true);
      setError(null);

      try {
        const hash = await walletClient.writeContract({
          address: CONTRACT_ADDRESS,
          abi: marchMadnessAbi,
          functionName: "updateBracket",
          args: [bracketHex],
        });
        setExistingBracket(bracketHex);
        return hash;
      } catch (err) {
        const msg =
          err instanceof Error ? err.message : "Failed to update bracket";
        setError(msg);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [walletClient],
  );

  // Set tag (regular write, not shielded)
  const setTag = useCallback(
    async (tag: string) => {
      if (!walletClient) throw new Error("Wallet not connected");
      setIsLoading(true);
      setError(null);

      try {
        const hash = await walletClient.writeContract({
          address: CONTRACT_ADDRESS,
          abi: marchMadnessAbi,
          functionName: "setTag",
          args: [tag],
        });
        return hash;
      } catch (err) {
        const msg =
          err instanceof Error ? err.message : "Failed to set tag";
        setError(msg);
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [walletClient],
  );

  return {
    entryCount,
    hasSubmitted,
    existingBracket,
    isLoading,
    error,
    isBeforeDeadline,
    submitBracket,
    updateBracket,
    setTag,
    fetchEntryCount,
    fetchMyBracket,
    walletAddress: walletClient?.account?.address ?? null,
  };
}

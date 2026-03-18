import { type ConnectedWallet } from "@privy-io/react-auth";
import { useSetActiveWallet } from "@privy-io/wagmi";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { BaseError, SwitchChainError, numberToHex, type Chain } from "viem";
import { useAccount } from "wagmi";

import { useDebugValueChanges } from "./useDebugValueChanges";
import { usePrivyWalletSelection } from "./usePrivyWalletSelection";
import { REQUIRED_CHAIN } from "../lib/config";
import {
  findWalletByAddress,
  isPrivyManagedWallet,
  normalizeAddress,
} from "../lib/privyWallets";

const parseCaipChainId = (chainId?: string | null): number | null => {
  if (!chainId) return null;
  const [, numericId] = chainId.split(":");
  const parsed = Number(numericId);
  return Number.isFinite(parsed) ? parsed : null;
};

const buildAddChainParams = (chain: Chain) => ({
  chainId: numberToHex(chain.id),
  chainName: chain.name,
  nativeCurrency: chain.nativeCurrency,
  rpcUrls: chain.rpcUrls.default.http,
  blockExplorerUrls: chain.blockExplorers?.default?.url
    ? [chain.blockExplorers.default.url]
    : undefined,
});

type RpcErrorLike = {
  code?: unknown;
  cause?: unknown;
  originalError?: unknown;
  data?: { originalError?: unknown };
};

const hasErrorCode = (error: unknown, code: number): boolean =>
  typeof error === "object" &&
  error !== null &&
  "code" in error &&
  (error as { code?: unknown }).code === code;

const findErrorByCode = (error: unknown, code: number): unknown | null => {
  if (!error) return null;
  if (hasErrorCode(error, code)) return error;

  if (error instanceof BaseError) {
    return error.walk((candidate) => hasErrorCode(candidate, code));
  }

  if (typeof error !== "object") return null;

  const candidate = error as RpcErrorLike;
  return (
    findErrorByCode(candidate.cause, code) ??
    findErrorByCode(candidate.originalError, code) ??
    findErrorByCode(candidate.data?.originalError, code)
  );
};

const isMissingChainError = (error: unknown): boolean =>
  findErrorByCode(error, SwitchChainError.code) !== null;

const extractErrorMessage = (error: unknown, fallback: string): string => {
  if (!error) return fallback;
  if (error instanceof Error) return error.message || fallback;
  if (typeof error === "string") return error;
  if (typeof error === "object" && error !== null && "message" in error) {
    const message = (error as { message?: unknown }).message;
    if (typeof message === "string" && message) return message;
  }
  return fallback;
};

const switchWalletChain = async (
  wallet: ConnectedWallet,
  chain: Chain,
): Promise<void> => {
  try {
    await wallet.switchChain(chain.id);
    return;
  } catch (error) {
    if (!isMissingChainError(error)) throw error;
  }

  const provider = await wallet.getEthereumProvider();
  await provider.request({
    method: "wallet_addEthereumChain",
    params: [buildAddChainParams(chain)],
  });
  await wallet.switchChain(chain.id);
};

export function useRequiredChain() {
  const { authenticated, preferredWallet, wallets, walletsReady } =
    usePrivyWalletSelection();
  const { setActiveWallet } = useSetActiveWallet();
  const { address, chainId: wagmiChainId } = useAccount();
  const [isSwitching, setIsSwitching] = useState(false);
  const [switchErrorState, setSwitchErrorState] = useState<{
    key: string | null;
    message: string | null;
  }>({ key: null, message: null });
  const lastWalletSyncRef = useRef<string | null>(null);

  const wagmiWallet = useMemo(
    () => findWalletByAddress(wallets, address),
    [address, wallets],
  );
  const activeWallet = useMemo(() => {
    if (preferredWallet) return preferredWallet;
    if (wagmiWallet) return wagmiWallet;
    return wallets.find(isPrivyManagedWallet) ?? null;
  }, [preferredWallet, wagmiWallet, wallets]);

  const isExternalActiveWallet =
    !!activeWallet && !isPrivyManagedWallet(activeWallet);
  const walletChainId = parseCaipChainId(activeWallet?.chainId);
  const activeChainId = isExternalActiveWallet ? wagmiChainId : walletChainId;
  const isOnRequiredChain = activeChainId === REQUIRED_CHAIN.id;
  const requiresChainSwitch =
    authenticated &&
    walletsReady &&
    isExternalActiveWallet &&
    activeChainId !== undefined &&
    activeChainId !== null &&
    !isOnRequiredChain;
  const switchErrorKey = `${normalizeAddress(activeWallet?.address) ?? "none"}:${activeWallet?.chainId ?? "none"}:${wagmiChainId ?? "none"}`;
  const switchError =
    switchErrorState.key === switchErrorKey ? switchErrorState.message : null;

  useDebugValueChanges("useRequiredChain", {
    authenticated,
    walletsReady,
    wagmiAddress: normalizeAddress(address),
    wagmiChainId: wagmiChainId ?? null,
    activeWalletAddress: normalizeAddress(activeWallet?.address),
    activeWalletChainId: activeWallet?.chainId ?? null,
    activeChainId: activeChainId ?? null,
    isExternalActiveWallet,
    isOnRequiredChain,
    requiresChainSwitch,
    switchError,
  });

  useEffect(() => {
    const activeWalletAddress = normalizeAddress(activeWallet?.address);
    if (!authenticated || !walletsReady || !activeWallet || !activeWalletAddress) {
      lastWalletSyncRef.current = null;
      return;
    }

    const wagmiAddress = normalizeAddress(address);
    const addressMismatch = activeWalletAddress !== wagmiAddress;
    const needsRequiredChainResync =
      isExternalActiveWallet && activeChainId === REQUIRED_CHAIN.id;

    if (!addressMismatch && !needsRequiredChainResync) {
      lastWalletSyncRef.current = null;
      return;
    }

    const syncKey = `${addressMismatch ? "align" : "required-chain"}:${activeWalletAddress}:${activeChainId ?? "unknown"}`;
    if (lastWalletSyncRef.current === syncKey) return;
    lastWalletSyncRef.current = syncKey;

    void setActiveWallet(activeWallet).catch(() => {
      lastWalletSyncRef.current = null;
    });
  }, [
    activeChainId,
    activeWallet,
    address,
    authenticated,
    isExternalActiveWallet,
    walletsReady,
    setActiveWallet,
  ]);

  const switchToRequiredChain = useCallback(async () => {
    if (!activeWallet || !isExternalActiveWallet) {
      return false;
    }

    setIsSwitching(true);
    setSwitchErrorState({ key: switchErrorKey, message: null });

    try {
      await switchWalletChain(activeWallet, REQUIRED_CHAIN);
      lastWalletSyncRef.current = null;
      await setActiveWallet(activeWallet);
      return true;
    } catch (error) {
      setSwitchErrorState({
        key: switchErrorKey,
        message: extractErrorMessage(
          error,
          `Failed to switch to ${REQUIRED_CHAIN.name}.`,
        ),
      });
      return false;
    } finally {
      setIsSwitching(false);
    }
  }, [activeWallet, isExternalActiveWallet, setActiveWallet, switchErrorKey]);

  return {
    activeWallet,
    activeChainId,
    isOnRequiredChain,
    isSwitchingChain: isSwitching,
    requiredChain: REQUIRED_CHAIN,
    requiresChainSwitch,
    switchChainError: switchError,
    switchToRequiredChain,
  };
}

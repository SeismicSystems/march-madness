import { usePrivy, useWallets, type ConnectedWallet } from "@privy-io/react-auth";
import { useSetActiveWallet } from "@privy-io/wagmi";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { BaseError, SwitchChainError, numberToHex, type Chain } from "viem";
import { useAccount } from "wagmi";

import { REQUIRED_CHAIN } from "../lib/config";

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

const isPrivyManagedWallet = (wallet: ConnectedWallet | null): boolean =>
  !!wallet &&
  (wallet.connectorType === "embedded" ||
    wallet.walletClientType === "privy" ||
    wallet.walletClientType === "privy-v2");

export function useRequiredChain() {
  const { authenticated } = usePrivy();
  const { wallets, ready } = useWallets();
  const { setActiveWallet } = useSetActiveWallet();
  const { address } = useAccount();
  const [isSwitching, setIsSwitching] = useState(false);
  const [switchError, setSwitchError] = useState<string | null>(null);
  const lastSyncedWalletRef = useRef<string | null>(null);

  const activeWallet = useMemo(() => {
    if (!address) return null;

    return (
      wallets.find(
        (wallet) => wallet.address.toLowerCase() === address.toLowerCase(),
      ) ?? null
    );
  }, [address, wallets]);

  const activeChainId = parseCaipChainId(activeWallet?.chainId);
  const isOnRequiredChain = activeChainId === REQUIRED_CHAIN.id;
  const isExternalActiveWallet =
    !!activeWallet && !isPrivyManagedWallet(activeWallet);
  const requiresChainSwitch =
    authenticated &&
    ready &&
    isExternalActiveWallet &&
    activeChainId !== null &&
    !isOnRequiredChain;

  useEffect(() => {
    if (!isExternalActiveWallet || !activeWallet) return;
    if (activeChainId !== REQUIRED_CHAIN.id) return;

    const syncKey = `${activeWallet.address.toLowerCase()}:${activeChainId}`;
    if (lastSyncedWalletRef.current === syncKey) return;
    lastSyncedWalletRef.current = syncKey;

    void setActiveWallet(activeWallet).catch(() => {
      lastSyncedWalletRef.current = null;
    });
  }, [activeChainId, activeWallet, isExternalActiveWallet, setActiveWallet]);

  useEffect(() => {
    setSwitchError(null);
  }, [activeWallet?.address, activeWallet?.chainId]);

  const switchToRequiredChain = useCallback(async () => {
    if (!activeWallet || !isExternalActiveWallet) {
      return false;
    }

    setIsSwitching(true);
    setSwitchError(null);

    try {
      await switchWalletChain(activeWallet, REQUIRED_CHAIN);
      lastSyncedWalletRef.current = null;
      await setActiveWallet(activeWallet);
      return true;
    } catch (error) {
      setSwitchError(
        extractErrorMessage(
          error,
          `Failed to switch to ${REQUIRED_CHAIN.name}.`,
        ),
      );
      return false;
    } finally {
      setIsSwitching(false);
    }
  }, [activeWallet, isExternalActiveWallet, setActiveWallet]);

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

import { useMemo } from "react";

import { useActiveWallet, usePrivy, useWallets } from "@privy-io/react-auth";

import { useDebugValueChanges } from "./useDebugValueChanges";
import {
  getPreferredPrivyWallet,
  normalizeAddress,
} from "../lib/privyWallets";

export function usePrivyWalletSelection() {
  const { authenticated, ready: privyReady, user } = usePrivy();
  const { wallet: privyActiveWallet } = useActiveWallet();
  const { ready: walletsReady, wallets } = useWallets();

  const preferredWallet = useMemo(() => {
    if (!authenticated || !privyReady || !walletsReady || !user) {
      return null;
    }

    return getPreferredPrivyWallet({
      authenticated,
      privyActiveWallet,
      user,
      wallets,
    });
  }, [
    authenticated,
    privyActiveWallet,
    privyReady,
    user,
    wallets,
    walletsReady,
  ]);

  useDebugValueChanges("usePrivyWalletSelection", {
    authenticated,
    privyReady,
    walletsReady,
    linkedWalletCount: user?.linkedAccounts?.filter(
      (account) => account.type === "wallet",
    ).length ?? 0,
    privyActiveWalletAddress:
      privyActiveWallet?.type === "ethereum"
        ? normalizeAddress(privyActiveWallet.address)
        : null,
    privyUserWalletAddress: normalizeAddress(user?.wallet?.address),
    preferredWalletAddress: normalizeAddress(preferredWallet?.address),
    preferredWalletChainId: preferredWallet?.chainId ?? null,
    walletCount: wallets.length,
  });

  return {
    authenticated,
    preferredWallet,
    preferredWalletAddress: normalizeAddress(preferredWallet?.address),
    privyReady,
    user,
    wallets,
    walletsReady,
  };
}

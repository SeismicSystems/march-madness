import { useMemo } from "react";

import { useActiveWallet, usePrivy, useWallets } from "@privy-io/react-auth";

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

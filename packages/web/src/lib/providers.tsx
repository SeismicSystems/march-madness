import React, { useEffect, useMemo, useRef } from "react";
import { ShieldedWalletProvider } from "seismic-react";
import { http } from "viem";

import {
  PrivyProvider,
  useActiveWallet,
  usePrivy,
  useWallets,
  type ConnectedWallet,
} from "@privy-io/react-auth";
import { WagmiProvider, useSetActiveWallet } from "@privy-io/wagmi";
import { QueryClientProvider } from "@tanstack/react-query";
import { useDisconnect } from "wagmi";

import { APP_CHAINS, REQUIRED_CHAIN, config, queryClient } from "./config";

const normalizeAddress = (address?: string | null) => address?.toLowerCase();

const findWalletByAddress = (
  wallets: ConnectedWallet[],
  address?: string | null,
): ConnectedWallet | null => {
  const normalizedAddress = normalizeAddress(address);
  if (!normalizedAddress) return null;

  return (
    wallets.find(
      (wallet) => normalizeAddress(wallet.address) === normalizedAddress,
    ) ?? null
  );
};

const isPrivyManagedWallet = (
  wallet?: {
    connectorType?: string;
    walletClientType?: string;
  } | null,
): boolean =>
  !!wallet &&
  (wallet.connectorType === "embedded" ||
    wallet.walletClientType === "privy" ||
    wallet.walletClientType === "privy-v2");

const getEmbeddedWalletAddresses = (
  linkedAccounts?: NonNullable<ReturnType<typeof usePrivy>["user"]>["linkedAccounts"],
): string[] =>
  (linkedAccounts ?? [])
    .filter(
      (account): account is Extract<typeof account, { type: "wallet" }> =>
        account.type === "wallet" && isPrivyManagedWallet(account),
    )
    .map((account) => account.address);

export const Providers: React.FC<React.PropsWithChildren> = ({ children }) => {
  const publicTransport = http(import.meta.env.VITE_RPC_URL);

  return (
    <PrivyProvider
      appId={import.meta.env.VITE_PRIVY_APP_ID || "placeholder-app-id"}
      config={{
        supportedChains: APP_CHAINS,
        defaultChain: REQUIRED_CHAIN,
        loginMethods: [
          "wallet",
          "email",
          "sms",
          "google",
          "twitter",
          "discord",
          "github",
          "passkey",
        ],
        embeddedWallets: {
          ethereum: {
            createOnLogin: "all-users",
          },
        },
        appearance: {
          theme: "dark",
          accentColor: "#825A6D",
        },
      }}
    >
      <QueryClientProvider client={queryClient}>
        <PrivyBackedWagmiProvider publicTransport={publicTransport}>
          {children}
        </PrivyBackedWagmiProvider>
      </QueryClientProvider>
    </PrivyProvider>
  );
};

function PrivyBackedWagmiProvider({
  children,
  publicTransport,
}: React.PropsWithChildren<{
  publicTransport: ReturnType<typeof http>;
}>) {
  return (
    <WagmiProvider config={config} reconnectOnMount={false}>
      <PrivyWalletSync />
      <ShieldedWalletProvider
        config={config}
        options={{ publicChain: REQUIRED_CHAIN, publicTransport }}
      >
        {children}
      </ShieldedWalletProvider>
    </WagmiProvider>
  );
}

function PrivyWalletSync() {
  const { authenticated, ready: privyReady, user } = usePrivy();
  const { wallet: privyActiveWallet } = useActiveWallet();
  const { ready: walletsReady, wallets } = useWallets();
  const { setActiveWallet } = useSetActiveWallet();
  const { disconnect } = useDisconnect();
  const lastSyncRef = useRef<string | null>(null);

  const preferredWallet = useMemo(() => {
    if (!privyReady || !walletsReady || !authenticated || !user) {
      return null;
    }

    if (privyActiveWallet?.type === "ethereum") {
      const activeWallet = findWalletByAddress(wallets, privyActiveWallet.address);
      if (activeWallet) return activeWallet;
    }

    const embeddedWalletAddresses = getEmbeddedWalletAddresses(user.linkedAccounts);
    for (const address of embeddedWalletAddresses) {
      const embeddedWallet = findWalletByAddress(wallets, address);
      if (embeddedWallet) return embeddedWallet;
    }

    const anyEmbeddedWallet = wallets.find(isPrivyManagedWallet);
    if (anyEmbeddedWallet) return anyEmbeddedWallet;

    return findWalletByAddress(wallets, user.wallet?.address);
  }, [
    authenticated,
    privyActiveWallet,
    privyReady,
    user,
    wallets,
    walletsReady,
  ]);

  useEffect(() => {
    if (!privyReady || !walletsReady) return;

    if (!authenticated || !user) {
      if (lastSyncRef.current === "disconnected") return;
      lastSyncRef.current = "disconnected";
      disconnect();
      return;
    }

    if (!preferredWallet) {
      lastSyncRef.current = null;
      return;
    }

    const syncKey = `${preferredWallet.address.toLowerCase()}:${preferredWallet.chainId ?? "unknown"}`;
    if (lastSyncRef.current === syncKey) return;
    lastSyncRef.current = syncKey;

    void setActiveWallet(preferredWallet).catch(() => {
      lastSyncRef.current = null;
    });
  }, [
    authenticated,
    disconnect,
    preferredWallet,
    privyReady,
    setActiveWallet,
    user,
    walletsReady,
  ]);

  return null;
}

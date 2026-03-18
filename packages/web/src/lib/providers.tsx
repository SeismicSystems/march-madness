import React, { useEffect, useRef } from "react";
import { ShieldedWalletProvider } from "seismic-react";
import { http } from "viem";

import { PrivyProvider } from "@privy-io/react-auth";
import { WagmiProvider, useSetActiveWallet } from "@privy-io/wagmi";
import { QueryClientProvider } from "@tanstack/react-query";
import { useDisconnect } from "wagmi";

import { usePrivyWalletSelection } from "../hooks/usePrivyWalletSelection";
import { APP_CHAINS, REQUIRED_CHAIN, config, queryClient } from "./config";

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
  const { authenticated, preferredWallet, privyReady, user, walletsReady } =
    usePrivyWalletSelection();
  const { setActiveWallet } = useSetActiveWallet();
  const { disconnect } = useDisconnect();
  const lastSyncRef = useRef<string | null>(null);

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

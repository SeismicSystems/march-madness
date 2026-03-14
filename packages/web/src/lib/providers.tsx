import React from "react";
import { ShieldedWalletProvider } from "seismic-react";
import { http } from "viem";

import { PrivyProvider } from "@privy-io/react-auth";
import { WagmiProvider } from "@privy-io/wagmi";
import { QueryClientProvider } from "@tanstack/react-query";

import { CHAINS, config, queryClient } from "./config";

export const Providers: React.FC<React.PropsWithChildren> = ({ children }) => {
  const publicChain = CHAINS[0];
  const publicTransport = http(import.meta.env.VITE_PUBLIC_RPC_URL);

  return (
    <PrivyProvider
      appId={import.meta.env.VITE_PRIVY_APP_ID || "placeholder-app-id"}
      config={{
        supportedChains: CHAINS,
        defaultChain: CHAINS[0],
        loginMethods: ["twitter", "discord"],
        embeddedWallets: {
          ethereum: {
            createOnLogin: "all-users",
          },
        },
        externalWallets: {
          walletConnect: {
            enabled: false,
          },
          disableAllExternalWallets: true,
        },
        appearance: {
          theme: "dark",
          accentColor: "#6366f1",
        },
      }}
    >
      <QueryClientProvider client={queryClient}>
        <WagmiProvider config={config} reconnectOnMount={true}>
          <ShieldedWalletProvider
            config={config}
            options={{ publicChain, publicTransport }}
          >
            {children}
          </ShieldedWalletProvider>
        </WagmiProvider>
      </QueryClientProvider>
    </PrivyProvider>
  );
};

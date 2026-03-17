import React from "react";
import { ShieldedWalletProvider } from "seismic-react";
import { http } from "viem";

import { PrivyProvider } from "@privy-io/react-auth";
import { WagmiProvider } from "@privy-io/wagmi";
import { QueryClientProvider } from "@tanstack/react-query";

import { CHAINS, config, queryClient } from "./config";

export const Providers: React.FC<React.PropsWithChildren> = ({ children }) => {
  const publicChain = CHAINS[0];
  const publicTransport = http(import.meta.env.VITE_RPC_URL);

  return (
    <PrivyProvider
      appId={import.meta.env.VITE_PRIVY_APP_ID || "placeholder-app-id"}
      config={{
        supportedChains: CHAINS,
        defaultChain: CHAINS[0],
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

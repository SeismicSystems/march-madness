import React from "react";
import { ShieldedWalletProvider } from "seismic-react";
import { http } from "viem";

import {
  PrivyProvider,
  type ConnectedWallet,
  type User,
} from "@privy-io/react-auth";
import { WagmiProvider } from "@privy-io/wagmi";
import { QueryClientProvider } from "@tanstack/react-query";

import { APP_CHAINS, REQUIRED_CHAIN, config, queryClient } from "./config";

const normalizeAddress = (address?: string | null) => address?.toLowerCase();

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

const pickActiveWalletForWagmi = ({
  wallets,
  user,
}: {
  wallets: ConnectedWallet[];
  user: User | null;
}): ConnectedWallet | undefined => {
  const verifiedAddress = normalizeAddress(user?.wallet?.address);
  const verifiedWallet = verifiedAddress
    ? wallets.find(
        (wallet) => normalizeAddress(wallet.address) === verifiedAddress,
      )
    : undefined;

  if (verifiedWallet) return verifiedWallet;
  if (isPrivyManagedWallet(user?.wallet)) {
    return wallets.find(isPrivyManagedWallet) ?? wallets[0];
  }

  return wallets.find((wallet) => !isPrivyManagedWallet(wallet)) ?? wallets[0];
};

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
        <WagmiProvider
          config={config}
          reconnectOnMount={true}
          setActiveWalletForWagmi={pickActiveWalletForWagmi}
        >
          <ShieldedWalletProvider
            config={config}
            options={{ publicChain: REQUIRED_CHAIN, publicTransport }}
          >
            {children}
          </ShieldedWalletProvider>
        </WagmiProvider>
      </QueryClientProvider>
    </PrivyProvider>
  );
};

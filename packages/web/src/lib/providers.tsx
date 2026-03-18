import React, { useCallback } from "react";
import { ShieldedWalletProvider } from "seismic-react";
import { http } from "viem";

import {
  PrivyProvider,
  useActiveWallet,
  usePrivy,
  type ConnectedWallet,
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
  const { user } = usePrivy();
  const { wallet: privyActiveWallet } = useActiveWallet();

  const pickActiveWalletForWagmi = useCallback(
    ({ wallets }: { wallets: ConnectedWallet[] }) => {
      const activeAddress =
        privyActiveWallet?.type === "ethereum"
          ? normalizeAddress(privyActiveWallet.address)
          : null;
      const activeWallet = activeAddress
        ? wallets.find(
            (wallet) => normalizeAddress(wallet.address) === activeAddress,
          )
        : undefined;
      if (activeWallet) return activeWallet;

      const embeddedWalletAddresses =
        getEmbeddedWalletAddresses(user?.linkedAccounts).map(normalizeAddress);
      const embeddedWallet = embeddedWalletAddresses
        .map((address: string | undefined) =>
          wallets.find((wallet) => normalizeAddress(wallet.address) === address),
        )
        .find((wallet): wallet is ConnectedWallet => !!wallet);
      if (embeddedWallet) return embeddedWallet;

      const anyEmbeddedWallet = wallets.find(isPrivyManagedWallet);
      if (anyEmbeddedWallet) return anyEmbeddedWallet;

      const verifiedAddress = normalizeAddress(user?.wallet?.address);
      const verifiedWallet = verifiedAddress
        ? wallets.find(
            (wallet) => normalizeAddress(wallet.address) === verifiedAddress,
          )
        : undefined;
      return verifiedWallet ?? wallets[0];
    },
    [privyActiveWallet, user?.linkedAccounts, user?.wallet?.address],
  );

  return (
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
  );
}

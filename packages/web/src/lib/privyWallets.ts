import type { ConnectedWallet } from "@privy-io/react-auth";

type PrivyWalletLike = {
  address?: string | null;
  chainId?: string | null;
  connectorType?: string;
  type?: string;
  walletClientType?: string;
} | null;

type LinkedAccountLike = {
  type: string;
  address?: string | null;
  connectorType?: string;
  walletClientType?: string;
} | null;

type PrivyUserLike = {
  linkedAccounts?: LinkedAccountLike[] | null;
  wallet?: { address?: string | null } | null;
} | null;

export const normalizeAddress = (address?: string | null): string | null =>
  address?.toLowerCase() ?? null;

export const isPrivyManagedWallet = (wallet?: PrivyWalletLike): boolean =>
  !!wallet &&
  (wallet.connectorType === "embedded" ||
    wallet.walletClientType === "privy" ||
    wallet.walletClientType === "privy-v2");

export const getEmbeddedWalletAddresses = (
  linkedAccounts?: LinkedAccountLike[] | null,
): string[] =>
  (linkedAccounts ?? [])
    .filter(
      (account): account is NonNullable<LinkedAccountLike> =>
        !!account &&
        account.type === "wallet" &&
        isPrivyManagedWallet(account),
    )
    .map((account) => account.address)
    .filter((address): address is string => !!address);

export const findWalletByAddress = (
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

export const getPreferredPrivyWallet = ({
  authenticated,
  privyActiveWallet,
  user,
  wallets,
}: {
  authenticated: boolean;
  privyActiveWallet?: PrivyWalletLike;
  user?: PrivyUserLike;
  wallets: ConnectedWallet[];
}): ConnectedWallet | null => {
  if (!authenticated || !user) {
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
};

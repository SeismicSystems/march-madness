import { sanvil, seismicTestnetGcp2 } from "seismic-viem";
import { http } from "viem";

import { createConfig } from "@privy-io/wagmi";
import { QueryClient } from "@tanstack/react-query";

export const queryClient = new QueryClient();

const parseChainId = (): number => {
  const chainId = import.meta.env.VITE_CHAIN_ID;
  if (!chainId) {
    // Default to sanvil for local dev
    return sanvil.id;
  }
  return parseInt(chainId);
};

const seismicTestnet = seismicTestnetGcp2;

const CHAIN_ID = parseChainId();
const ENABLED_CHAINS = [sanvil, seismicTestnet];
export const CHAINS = ENABLED_CHAINS.filter(({ id }) => id === CHAIN_ID);

// Fallback to sanvil if no chain matched
export const APP_CHAINS = CHAINS.length > 0 ? CHAINS : [sanvil];
export const REQUIRED_CHAIN = APP_CHAINS[0];

export const config = createConfig({
  // @ts-expect-error: privy wagmi typing mismatch
  chains: APP_CHAINS,
  transports: {
    [sanvil.id]: http(),
    [seismicTestnet.id]: http(import.meta.env.VITE_RPC_URL, {
      fetchOptions: { cache: "no-store" },
    }),
  },
  ssr: false,
});

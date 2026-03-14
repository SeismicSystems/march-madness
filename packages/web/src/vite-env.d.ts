/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_PRIVY_APP_ID: string;
  readonly VITE_CONTRACT_ADDRESS?: string; // injected by populate script for local dev only
  readonly VITE_CHAIN_ID: string;
  readonly VITE_RPC_URL: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

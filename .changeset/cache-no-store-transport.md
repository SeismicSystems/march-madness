---
"@march-madness/web": patch
---

Add `cache: "no-store"` fetch option to both HTTP transports (wagmi config in `config.ts` and seismic-react public transport in `providers.tsx`) to prevent browser/proxy caching of RPC responses. Users reported stale `recentBlockHash` values in shielded transactions; this ensures `eth_getBlockByNumber("latest")` always bypasses any intermediate cache. Companion fix in seismic-viem: SeismicSystems/seismic#123.

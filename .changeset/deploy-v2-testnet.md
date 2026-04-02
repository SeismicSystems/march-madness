---
"march-madness": patch
---

Deploy MarchMadnessV2 and BracketGroupsV2 to testnet (chain 5124). Enable Solidity optimizer (200 runs) in foundry.toml — required to keep BracketGroupsV2 under the EIP-170 24576-byte limit. Fix deploy-v2.sh to invoke sforge directly without mise.

#!/usr/bin/env bash
set -euo pipefail

# Deploy MarchMadnessV2 + BracketGroupsV2 (migration cutover contracts) to testnet.
# Writes both addresses to data/deployments.json under year/chainId/v2.
#
# Usage:
#   ./scripts/deploy-v2.sh

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEPLOYMENTS="$REPO_ROOT/data/deployments.json"
YEAR="2026"
CHAIN_ID="5124"  # seismicTestnetGcp2

# ── Load .env ──────────────────────────────────────────────
if [[ -f "$REPO_ROOT/.env" ]]; then
  set -a
  source "$REPO_ROOT/.env"
  set +a
fi

: "${DEPLOYER_PRIVATE_KEY:?Set DEPLOYER_PRIVATE_KEY in .env}"
: "${VITE_RPC_URL:?Set VITE_RPC_URL in .env}"

SFORGE="${SFORGE:-sforge}"
SSOLC="${SSOLC:-$(command -v ssolc 2>/dev/null || echo "ssolc")}"

echo "Deploying MarchMadnessV2 + BracketGroupsV2 to ${VITE_RPC_URL}..."
OUTPUT=$(cd "$REPO_ROOT/contracts" && PATH="$HOME/.seismic/bin:$PATH" FOUNDRY_SOLC="$SSOLC" "$SFORGE" \
  script script/DeployV2.s.sol \
  --rpc-url "$VITE_RPC_URL" \
  --broadcast \
  --private-key "$DEPLOYER_PRIVATE_KEY" < /dev/null 2>&1) || { echo "$OUTPUT"; exit 1; }

echo "$OUTPUT"

MMV2_ADDRESS=$(echo "$OUTPUT" | grep -oP 'MarchMadnessV2 deployed at:\s+\K0x[0-9a-fA-F]{40}')
BGV2_ADDRESS=$(echo "$OUTPUT" | grep -oP 'BracketGroupsV2 deployed at:\s+\K0x[0-9a-fA-F]{40}')

if [[ -z "$MMV2_ADDRESS" ]]; then
  echo "ERROR: Could not parse MarchMadnessV2 address from sforge output" >&2
  exit 1
fi

echo ""
echo "MarchMadnessV2:  $MMV2_ADDRESS"
echo "BracketGroupsV2: $BGV2_ADDRESS"

# Write to deployments.json under year/chainId/v2
bun -e "
  const fs = require('fs');
  const d = JSON.parse(fs.readFileSync('$DEPLOYMENTS', 'utf-8'));
  d['$YEAR'] = d['$YEAR'] || {};
  d['$YEAR']['$CHAIN_ID'] = d['$YEAR']['$CHAIN_ID'] || {};
  d['$YEAR']['$CHAIN_ID']['v2'] = {
    marchMadness: '$MMV2_ADDRESS',
    bracketGroups: '${BGV2_ADDRESS:-}'
  };
  fs.writeFileSync('$DEPLOYMENTS', JSON.stringify(d, null, 2) + '\n');
"

echo ""
echo "Updated $DEPLOYMENTS"
cat "$DEPLOYMENTS"
echo ""
echo "Next steps:"
echo "  1. Fund MarchMadnessV2 with the V1 prize pool balance:"
echo "       cast send $MMV2_ADDRESS 'fund()' --value <amount> --private-key \$DEPLOYER_PRIVATE_KEY"
echo "  2. Run the entry migration:"
echo "       bun migrate:entries -- --old-mm <V1_ADDRESS> --new-mm $MMV2_ADDRESS --private-key \$DEPLOYER_PRIVATE_KEY"
echo "  3. git add $DEPLOYMENTS && git commit -m 'deploy: MarchMadnessV2 + BracketGroupsV2'"

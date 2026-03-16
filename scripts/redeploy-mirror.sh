#!/usr/bin/env bash
set -euo pipefail

# Redeploy BracketMirror only to testnet and update data/deployments.json.
# Used when BracketMirror.sol changes but MarchMadness + BracketGroups stay the same.
#
# Usage:
#   ./scripts/redeploy-mirror.sh

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

echo "Deploying BracketMirror to ${VITE_RPC_URL}..."
OUTPUT=$(cd "$REPO_ROOT/contracts" && mise run sforge -- \
  script script/DeployMirror.s.sol \
  --rpc-url "$VITE_RPC_URL" \
  --broadcast \
  --private-key "$DEPLOYER_PRIVATE_KEY" 2>&1)

echo "$OUTPUT"

# Parse address from output
BM_ADDRESS=$(echo "$OUTPUT" | grep -oP 'BracketMirror deployed at:\s+\K0x[0-9a-fA-F]{40}')

if [[ -z "$BM_ADDRESS" ]]; then
  echo "ERROR: Could not parse BracketMirror address from sforge output" >&2
  exit 1
fi

echo ""
echo "BracketMirror: $BM_ADDRESS"

# ── Update deployments.json (only bracketMirror field) ─────
bun -e "
  const fs = require('fs');
  const d = JSON.parse(fs.readFileSync('$DEPLOYMENTS', 'utf-8'));
  d['$YEAR'] = d['$YEAR'] || {};
  d['$YEAR']['$CHAIN_ID'] = d['$YEAR']['$CHAIN_ID'] || {};
  d['$YEAR']['$CHAIN_ID'].bracketMirror = '$BM_ADDRESS';
  fs.writeFileSync('$DEPLOYMENTS', JSON.stringify(d, null, 2) + '\n');
"

echo ""
echo "Updated $DEPLOYMENTS"
cat "$DEPLOYMENTS"
echo ""
echo "Next steps:"
echo "  1. git add $DEPLOYMENTS && git commit -m 'Redeploy BracketMirror'"
echo "  2. git push"

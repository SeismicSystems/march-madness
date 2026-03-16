#!/usr/bin/env bash
set -euo pipefail

# Redeploy BracketGroups only to testnet against an existing MarchMadness contract,
# and update only the bracketGroups field in data/deployments.json.
#
# Usage:
#   ./scripts/redeploy-bracket-groups.sh --march-madness-address 0x...

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

# ── Parse args ─────────────────────────────────────────────
MARCH_MADNESS_ADDRESS=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --march-madness-address)
      MARCH_MADNESS_ADDRESS="$2"
      shift 2
      ;;
    *)
      echo "Usage: $0 --march-madness-address 0x..." >&2
      exit 1
      ;;
  esac
done

: "${MARCH_MADNESS_ADDRESS:?Pass --march-madness-address 0x...}"
: "${DEPLOYER_PRIVATE_KEY:?Set DEPLOYER_PRIVATE_KEY in .env}"
: "${VITE_RPC_URL:?Set VITE_RPC_URL in .env}"

echo "Deploying BracketGroups to ${VITE_RPC_URL}..."
echo "Using MarchMadness: $MARCH_MADNESS_ADDRESS"
OUTPUT=$(cd "$REPO_ROOT/contracts" && MARCH_MADNESS_ADDRESS="$MARCH_MADNESS_ADDRESS" mise run sforge -- \
  script script/DeployBracketGroups.s.sol \
  --rpc-url "$VITE_RPC_URL" \
  --broadcast \
  --private-key "$DEPLOYER_PRIVATE_KEY" 2>&1)

echo "$OUTPUT"

# Parse address from output
BG_ADDRESS=$(echo "$OUTPUT" | grep -oP 'BracketGroups deployed at:\s+\K0x[0-9a-fA-F]{40}')

if [[ -z "$BG_ADDRESS" ]]; then
  echo "ERROR: Could not parse BracketGroups address from sforge output" >&2
  exit 1
fi

echo ""
echo "BracketGroups: $BG_ADDRESS"

# ── Update deployments.json (only bracketGroups field) ─────
bun -e "
  const fs = require('fs');
  const d = JSON.parse(fs.readFileSync('$DEPLOYMENTS', 'utf-8'));
  d['$YEAR'] = d['$YEAR'] || {};
  d['$YEAR']['$CHAIN_ID'] = d['$YEAR']['$CHAIN_ID'] || {};
  d['$YEAR']['$CHAIN_ID'].bracketGroups = '$BG_ADDRESS';
  fs.writeFileSync('$DEPLOYMENTS', JSON.stringify(d, null, 2) + '\n');
"

echo ""
echo "Updated $DEPLOYMENTS"
cat "$DEPLOYMENTS"
echo ""
echo "Next steps:"
echo "  1. git add $DEPLOYMENTS && git commit -m 'Redeploy BracketGroups'"
echo "  2. git push"

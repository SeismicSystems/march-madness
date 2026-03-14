#!/usr/bin/env bash
set -euo pipefail

# Deploy MarchMadness to testnet and write the address to data/deployments.json.
#
# Usage:
#   ./scripts/deploy-testnet.sh                          # deploy + write address
#   ./scripts/deploy-testnet.sh --contract-address 0x... # skip deploy, just write address

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
CONTRACT_ADDRESS=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --contract-address)
      CONTRACT_ADDRESS="$2"
      shift 2
      ;;
    *)
      echo "Usage: $0 [--contract-address 0x...]" >&2
      exit 1
      ;;
  esac
done

# ── Deploy (unless address provided) ──────────────────────
if [[ -z "$CONTRACT_ADDRESS" ]]; then
  : "${DEPLOYER_PRIVATE_KEY:?Set DEPLOYER_PRIVATE_KEY in .env}"
  : "${VITE_RPC_URL:?Set VITE_RPC_URL in .env}"

  echo "Deploying MarchMadness to ${VITE_RPC_URL}..."
  OUTPUT=$(cd "$REPO_ROOT/contracts" && mise run sforge -- \
    script script/MarchMadness.s.sol \
    --rpc-url "$VITE_RPC_URL" \
    --broadcast \
    --private-key "$DEPLOYER_PRIVATE_KEY" 2>&1)

  echo "$OUTPUT"

  CONTRACT_ADDRESS=$(echo "$OUTPUT" | grep -oP 'deployed at:\s+\K0x[0-9a-fA-F]{40}')
  if [[ -z "$CONTRACT_ADDRESS" ]]; then
    echo "ERROR: Could not parse contract address from sforge output" >&2
    exit 1
  fi
fi

echo ""
echo "Contract address: $CONTRACT_ADDRESS"

# ── Write to deployments.json ─────────────────────────────
# Use bun to update JSON (no jq dependency)
bun -e "
  const fs = require('fs');
  const d = JSON.parse(fs.readFileSync('$DEPLOYMENTS', 'utf-8'));
  d['$YEAR'] = d['$YEAR'] || {};
  d['$YEAR']['$CHAIN_ID'] = '$CONTRACT_ADDRESS';
  fs.writeFileSync('$DEPLOYMENTS', JSON.stringify(d, null, 2) + '\n');
"

echo "Updated $DEPLOYMENTS"
cat "$DEPLOYMENTS"
echo ""
echo "Next steps:"
echo "  1. git add $DEPLOYMENTS && git commit -m 'Deploy to testnet: $CONTRACT_ADDRESS'"
echo "  2. git push"

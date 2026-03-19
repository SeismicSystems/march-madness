#!/usr/bin/env bash
set -euo pipefail

# Mirror Yahoo Fantasy bracket group to BracketMirror contract.
#
# Usage:
#   ./scripts/mirror-yahoo.sh --group-id 21309 [--slug my-league]
#
# Runs:
#   1. Rust mirror-importer (fetches Yahoo API → platform.json)
#   2. Bun yahoo-mirror script (reads platform.json → creates/updates on-chain mirror)

GROUP_ID=""
SLUG=""
EXTRA_RUST_ARGS=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --group-id)
            GROUP_ID="$2"
            shift 2
            ;;
        --slug)
            SLUG="$2"
            shift 2
            ;;
        --force-refresh)
            EXTRA_RUST_ARGS="$EXTRA_RUST_ARGS --force-refresh"
            shift
            ;;
        *)
            echo "Unknown arg: $1"
            exit 1
            ;;
    esac
done

if [[ -z "$GROUP_ID" ]]; then
    echo "Usage: $0 --group-id <id> [--slug <slug>] [--force-refresh]"
    exit 1
fi

RUST_ARGS="--group-id $GROUP_ID"
if [[ -n "$SLUG" ]]; then
    RUST_ARGS="$RUST_ARGS --slug $SLUG"
fi
RUST_ARGS="$RUST_ARGS$EXTRA_RUST_ARGS"

echo "=== Step 1: Fetch Yahoo data → platform.json ==="
# shellcheck disable=SC2086
cargo run -p mirror-importer -- $RUST_ARGS

echo ""
echo "=== Step 2: Mirror to chain ==="
bun run --filter @march-madness/localdev yahoo-mirror -- --group-id "$GROUP_ID"

echo ""
echo "=== Done ==="

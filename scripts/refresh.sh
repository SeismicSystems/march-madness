#!/usr/bin/env bash
#
# Run the KenPom/Kalshi ingestion pipeline (steps 1-5).
#
# Usage:
#   ./scripts/refresh.sh              # 6-hour cache (default)
#   ./scripts/refresh.sh --hours 0    # bypass cache
#   ./scripts/refresh.sh --hours 12   # 12-hour cache
#
set -euo pipefail

CACHE_HOURS=6

while [[ $# -gt 0 ]]; do
    case "$1" in
        --hours)
            CACHE_HOURS="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1" >&2
            echo "Usage: $0 [--hours <cache_hours>]" >&2
            exit 1
            ;;
    esac
done

CACHE_TTL=$((CACHE_HOURS * 3600))

echo "=== Pipeline: cache=${CACHE_HOURS}h (${CACHE_TTL}s) ==="
echo

# 1. Scrape KenPom ratings
echo "--- Step 1/5: Scrape KenPom ratings ---"
uv run scripts/scrape_kenpom.py --bracket-only
echo

# 2. Fetch raw Kalshi futures
echo "--- Step 2/5: Fetch raw Kalshi futures ---"
cargo run --release -p kalshi --bin kalshi -- fetch --raw --cache-ttl "$CACHE_TTL"
echo

# 3. Fit KenPom anchor model
echo "--- Step 3/5: Fit KenPom anchor model ---"
uv run scripts/fit_kenpom_model.py
echo

# 4. Fetch & normalize Kalshi futures
echo "--- Step 4/5: Normalize Kalshi futures ---"
cargo run --release -p kalshi --bin kalshi -- fetch --cache-ttl "$CACHE_TTL"
echo

# 5. Calibrate simulation goose values
echo "--- Step 5/5: Calibrate goose values ---"
cargo run --release -p bracket-sim --bin calibrate
echo

echo "=== Pipeline complete ==="

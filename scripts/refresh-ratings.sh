#!/usr/bin/env bash
set -euo pipefail

# Scrape fresh KenPom ratings and then run Kalshi calibration to update goose values.
#
# Usage:
#   ./scripts/refresh-ratings.sh                       # scrape + calibrate (2h cache TTL)
#   ./scripts/refresh-ratings.sh --cache-ttl 3600      # custom Kalshi cache TTL (seconds)
#   ./scripts/refresh-ratings.sh --no-kalshi           # scrape kenpom only, skip calibration
#   ./scripts/refresh-ratings.sh --no-kenpom           # calibrate only, skip kenpom scrape
#   ./scripts/refresh-ratings.sh -- --max-iter 200     # pass extra flags to calibrator
#
# Everything after "--" is forwarded to the calibrate binary.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

CACHE_TTL="7200"  # 2 hours
RUN_KENPOM=true
RUN_KALSHI=true
CALIBRATE_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --cache-ttl)
      CACHE_TTL="$2"
      shift 2
      ;;
    --no-kalshi)
      RUN_KALSHI=false
      shift
      ;;
    --no-kenpom)
      RUN_KENPOM=false
      shift
      ;;
    --)
      shift
      CALIBRATE_ARGS=("$@")
      break
      ;;
    *)
      echo "Unknown option: $1" >&2
      echo "Usage: $0 [--cache-ttl SECONDS] [--no-kalshi] [--no-kenpom] [-- CALIBRATE_ARGS...]" >&2
      exit 1
      ;;
  esac
done

# ── Step 1/2: Scrape KenPom ─────────────────────────────────
if [[ "$RUN_KENPOM" == true ]]; then
  echo "[1/2] Scraping KenPom ratings..."
  if ! uv run scripts/scrape_kenpom.py --bracket-only; then
    echo "FAILED: KenPom scrape failed, aborting." >&2
    exit 1
  fi
  echo "      Wrote data/2026/men/kenpom.csv"
else
  echo "[1/2] Skipping KenPom scrape (--no-kenpom)"
fi

# ── Step 2/2: Kalshi calibration ────────────────────────────
if [[ "$RUN_KALSHI" == true ]]; then
  echo "[2/2] Running Kalshi calibration (cache-ttl=${CACHE_TTL}s)..."
  cargo run --release -p bracket-sim --bin calibrate -- \
    --cache-ttl "$CACHE_TTL" \
    "${CALIBRATE_ARGS[@]+"${CALIBRATE_ARGS[@]}"}"
  echo "      Calibrated ratings saved to data/2026/men/kenpom.csv"
else
  echo "[2/2] Skipping Kalshi calibration (--no-kalshi)"
fi

echo "Done."

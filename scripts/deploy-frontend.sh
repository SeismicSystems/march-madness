#!/usr/bin/env bash
# Zero-downtime frontend deploy.
#
# Builds to a timestamped directory, then atomically swaps a "current" symlink
# so nginx never serves from a half-built or empty directory.
#
# Usage: ./scripts/deploy-frontend.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WEB_DIR="$REPO_ROOT/packages/web"

cd "$REPO_ROOT"

# Pull latest code and install deps
git pull
bun install

# Build to a timestamped staging directory
STAMP="dist-$(date +%s)"
STAGE_DIR="$WEB_DIR/$STAMP"

echo "Building frontend to $STAMP..."
bun run --filter @march-madness/web build -- --outDir "$STAMP"

# Verify the build produced output
if [ ! -f "$STAGE_DIR/index.html" ]; then
  echo "ERROR: Build failed — $STAGE_DIR/index.html not found" >&2
  rm -rf "$STAGE_DIR"
  exit 1
fi

# Atomic symlink swap: create new symlink then rename over the old one.
# mv -T is atomic on Linux when src and dst are on the same filesystem.
ln -sfn "$STAMP" "$WEB_DIR/current-tmp"
mv -T "$WEB_DIR/current-tmp" "$WEB_DIR/current"

echo "Symlink swapped: current -> $STAMP"

# Clean up old dist-* directories (keep the one we just deployed)
find "$WEB_DIR" -maxdepth 1 -name 'dist-*' -not -name "$STAMP" -type d -exec rm -rf {} +

echo "Frontend deploy complete."

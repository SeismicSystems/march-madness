#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────────────
# Local CI — mirrors .github/workflows/ci.yml exactly.
# Run from repo root:  ./scripts/ci.sh [section]
#
# Sections (run individually or omit to run all):
#   contracts   — build, test, fmt check
#   packages    — typecheck, lint, build, test
#   crates      — build, test, fmt, clippy
#   changeset   — verify docs/changeset.md is modified (vs main)
#
# KEEP IN SYNC with .github/workflows/ci.yml — see CLAUDE.md rule #7.
# ──────────────────────────────────────────────────────────────────────
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

passed=()
failed=()
skipped=()

run_step() {
  local name="$1"
  shift
  printf "${CYAN}▸ %-40s${NC}" "$name"
  if "$@" > /tmp/ci-step-output.log 2>&1; then
    printf "${GREEN}PASS${NC}\n"
    passed+=("$name")
  else
    printf "${RED}FAIL${NC}\n"
    cat /tmp/ci-step-output.log
    failed+=("$name")
  fi
}

skip_step() {
  local name="$1"
  local reason="$2"
  printf "${YELLOW}▸ %-40s SKIP (%s)${NC}\n" "$name" "$reason"
  skipped+=("$name")
}

# ─── Changeset ────────────────────────────────────────────────────────
run_changeset() {
  echo ""
  echo "━━━ Changeset ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  if git diff --name-only main...HEAD 2>/dev/null | grep -q '^docs/changeset.md$'; then
    run_step "changeset modified" true
  else
    # On main itself, skip the check
    if [ "$(git rev-parse --abbrev-ref HEAD)" = "main" ]; then
      skip_step "changeset modified" "on main branch"
    else
      run_step "changeset modified" false
    fi
  fi
}

# ─── Contracts ────────────────────────────────────────────────────────
run_contracts() {
  echo ""
  echo "━━━ Contracts ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  if [ ! -f contracts/foundry.toml ]; then
    skip_step "contracts build" "contracts/foundry.toml not found"
    skip_step "contracts test" "contracts/foundry.toml not found"
    skip_step "contracts fmt" "contracts/foundry.toml not found"
    return
  fi
  if ! command -v sforge &>/dev/null; then
    skip_step "contracts build" "sforge not installed"
    skip_step "contracts test" "sforge not installed"
    skip_step "contracts fmt" "sforge not installed"
    return
  fi
  run_step "contracts build" sforge build --root contracts
  run_step "contracts test" sforge test -vv --root contracts
  run_step "contracts fmt" sforge fmt --check --root contracts
}

# ─── Packages ─────────────────────────────────────────────────────────
run_packages() {
  echo ""
  echo "━━━ Packages ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  if ! command -v bun &>/dev/null; then
    skip_step "packages install" "bun not installed"
    return
  fi
  if [ ! -f package.json ]; then
    skip_step "packages install" "no root package.json"
    return
  fi

  run_step "packages install" bun install --frozen-lockfile 2>/dev/null || bun install

  # Client
  if [ -f packages/client/package.json ]; then
    if grep -q '"typecheck"' packages/client/package.json 2>/dev/null; then
      run_step "client typecheck" bash -c "cd packages/client && bun run typecheck"
    fi
    if grep -q '"lint:check"' packages/client/package.json 2>/dev/null; then
      run_step "client lint" bash -c "cd packages/client && bun run lint:check"
    fi
    if grep -q '"build"' packages/client/package.json 2>/dev/null; then
      run_step "client build" bash -c "cd packages/client && bun run build"
    fi
    if grep -q '"test"' packages/client/package.json 2>/dev/null; then
      run_step "client test" bash -c "cd packages/client && bun test"
    fi
  else
    skip_step "client" "packages/client/package.json not found"
  fi

  # Web
  if [ -f packages/web/package.json ]; then
    if grep -q '"typecheck"' packages/web/package.json 2>/dev/null; then
      run_step "web typecheck" bash -c "cd packages/web && bun run typecheck"
    fi
    if grep -q '"lint:check"' packages/web/package.json 2>/dev/null; then
      run_step "web lint" bash -c "cd packages/web && bun run lint:check"
    fi
    if grep -q '"build"' packages/web/package.json 2>/dev/null; then
      run_step "web build" bash -c "cd packages/web && bun run build"
    fi
  else
    skip_step "web" "packages/web/package.json not found"
  fi
}

# ─── Crates ───────────────────────────────────────────────────────────
run_crates() {
  echo ""
  echo "━━━ Crates ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  if [ ! -f crates/Cargo.toml ]; then
    skip_step "crates build" "crates/Cargo.toml not found"
    skip_step "crates test" "crates/Cargo.toml not found"
    skip_step "crates fmt" "crates/Cargo.toml not found"
    skip_step "crates clippy" "crates/Cargo.toml not found"
    return
  fi
  if ! command -v cargo &>/dev/null; then
    skip_step "crates build" "cargo not installed"
    return
  fi
  run_step "crates build" cargo build --manifest-path crates/Cargo.toml
  run_step "crates test" cargo test --manifest-path crates/Cargo.toml
  run_step "crates fmt" cargo fmt --all --manifest-path crates/Cargo.toml -- --check
  run_step "crates clippy" cargo clippy --all-targets --manifest-path crates/Cargo.toml -- -D warnings
}

# ─── Summary ──────────────────────────────────────────────────────────
print_summary() {
  echo ""
  echo "━━━ Summary ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  [ ${#passed[@]} -gt 0 ] && printf "${GREEN}✓ Passed:  %d${NC}\n" "${#passed[@]}"
  [ ${#skipped[@]} -gt 0 ] && printf "${YELLOW}○ Skipped: %d${NC}\n" "${#skipped[@]}"
  [ ${#failed[@]} -gt 0 ] && printf "${RED}✗ Failed:  %d${NC}\n" "${#failed[@]}"
  for f in "${failed[@]}"; do
    printf "${RED}  ✗ %s${NC}\n" "$f"
  done
  echo ""
  if [ ${#failed[@]} -gt 0 ]; then
    printf "${RED}CI FAILED${NC}\n"
    exit 1
  else
    printf "${GREEN}CI PASSED${NC}\n"
  fi
}

# ─── Main ─────────────────────────────────────────────────────────────
section="${1:-all}"

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║              Local CI — March Madness on Seismic            ║"
echo "╚══════════════════════════════════════════════════════════════╝"

case "$section" in
  contracts)  run_contracts ;;
  packages)   run_packages ;;
  crates)     run_crates ;;
  changeset)  run_changeset ;;
  all)
    run_changeset
    run_contracts
    run_packages
    run_crates
    ;;
  *)
    echo "Usage: $0 [contracts|packages|crates|changeset|all]"
    exit 1
    ;;
esac

print_summary

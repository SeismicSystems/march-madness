#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────────────
# Local CI — mirrors .github/workflows/ci.yml exactly.
# Run from repo root:  ./scripts/ci.sh [section]
#
# Uses mise for tool management (sfoundry, ssolc, bun) — same as GitHub CI.
# Requires: mise (https://mise.jdx.dev), cargo/rustup
#
# Sections (run individually or omit to run all):
#   contracts   — build, test, fmt check (via mise)
#   packages    — typecheck, lint, build, test (via mise)
#   crates      — build, test, fmt, clippy (cargo)
#   python      — deps install, script smoke tests (via uv)
#   changeset   — verify docs/changeset.md is modified (vs main)
#
# KEEP IN SYNC with .github/workflows/ci.yml — see CLAUDE.md rule #8.
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
  if [ "$(git rev-parse --abbrev-ref HEAD)" = "main" ]; then
    skip_step "changeset modified" "on main branch"
  elif git diff --name-only main...HEAD 2>/dev/null | grep -q '^docs/changeset.md$'; then
    run_step "changeset modified" true
  else
    run_step "changeset modified" false
  fi
}

# ─── Contracts (via mise — mirrors GitHub CI exactly) ─────────────────
run_contracts() {
  echo ""
  echo "━━━ Contracts ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  if [ ! -f contracts/foundry.toml ]; then
    skip_step "contracts" "contracts/foundry.toml not found"
    return
  fi
  if ! command -v mise &>/dev/null; then
    skip_step "contracts" "mise not installed (https://mise.jdx.dev)"
    return
  fi
  run_step "contracts build" bash -c "cd contracts && mise run build"
  run_step "contracts test" bash -c "cd contracts && mise run test"
  run_step "contracts fmt" bash -c "cd contracts && mise run fmt-check"
}

# ─── Packages (via mise — mirrors GitHub CI exactly) ──────────────────
run_packages() {
  echo ""
  echo "━━━ Packages ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  if ! command -v mise &>/dev/null; then
    skip_step "packages" "mise not installed (https://mise.jdx.dev)"
    return
  fi
  if [ ! -f packages/mise.toml ]; then
    skip_step "packages" "packages/mise.toml not found"
    return
  fi
  run_step "packages typecheck" bash -c "cd packages && mise run typecheck"
  run_step "packages lint" bash -c "cd packages && mise run lint::check"
  run_step "packages build" bash -c "cd packages && mise run build"
  run_step "packages test" bash -c "cd packages && mise run test"
}

# ─── Crates (cargo — no mise needed) ─────────────────────────────────
run_crates() {
  echo ""
  echo "━━━ Crates ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  if [ ! -f Cargo.toml ]; then
    run_step "crates" bash -c "echo 'Cargo.toml not found' && exit 1"
    return
  fi
  if ! command -v cargo &>/dev/null; then
    run_step "crates" bash -c "echo 'cargo not installed' && exit 1"
    return
  fi
  run_step "crates build" cargo build
  run_step "crates test" cargo test
  run_step "crates fmt" cargo fmt --all -- --check
  run_step "crates clippy" cargo clippy --all-targets -- -D warnings
}

# ─── Python (via uv) ─────────────────────────────────────────────────
run_python() {
  echo ""
  echo "━━━ Python ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  if [ ! -f pyproject.toml ]; then
    skip_step "python" "pyproject.toml not found"
    return
  fi
  if ! command -v uv &>/dev/null; then
    skip_step "python" "uv not installed (https://docs.astral.sh/uv/)"
    return
  fi
  if [ -f uv.lock ]; then
    run_step "python deps" uv sync --frozen
  else
    run_step "python deps" uv sync
  fi
  run_step "scrape_kenpom --help" uv run scripts/scrape_kenpom.py --help
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
  python)     run_python ;;
  changeset)  run_changeset ;;
  all)
    run_changeset
    run_contracts
    run_packages
    run_crates
    run_python
    ;;
  *)
    echo "Usage: $0 [contracts|packages|crates|python|changeset|all]"
    exit 1
    ;;
esac

print_summary

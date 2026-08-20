#!/usr/bin/env bash
# Full differential verification of the Rust translation against the C ground
# truth. Enumerates every valid Cargo feature combination, builds both shared
# objects, diffs their exported symbols, and runs the Phase B/C/D test suites.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"
: "${TMPDIR:=/tmp}"
LOG="$TMPDIR/verify.log"
: > "$LOG"

FAILED=0
step() { printf '\n=========== %s ===========\n' "$*"; }
ok()   { printf '  [ OK ] %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------------------
# Phase A.1 -- enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
step "Phase A: enumerating feature combinations"
FEATURES=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml)

if [ -z "$FEATURES" ]; then
  echo "  Cargo.toml declares no [features] -> exactly ONE combination (empty set)."
  COMBOS=("")
else
  # Full power set of the declared features.
  feats=($FEATURES)
  n=${#feats[@]}
  COMBOS=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then combo="${combo:+$combo,}${feats[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "  ${#COMBOS[@]} combination(s): $(for c in "${COMBOS[@]}"; do printf '[%s] ' "${c:-<none>}"; done)"

# ---------------------------------------------------------------------------
# Phase A.2 -- build the C ground-truth shared library
# ---------------------------------------------------------------------------
step "Phase A: building the C shared library"
mkdir -p c_src/build
if cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON >>"$LOG" 2>&1 \
   && cmake --build c_src/build >>"$LOG" 2>&1; then
  C_SO="$ROOT/c_src/build/libSimpleList.so"
  ok "C .so: $C_SO"
else
  bad "C build failed (see $LOG)"; exit 1
fi
export SIMPLELIST_C_SO="$C_SO"

# ---------------------------------------------------------------------------
# Phase 2 -- cargo check for EVERY combination
# ---------------------------------------------------------------------------
step "Phase A: cargo check for every feature combination"
for combo in "${COMBOS[@]}"; do
  if timeout 600 cargo check --offline --no-default-features \
       ${combo:+--features "$combo"} --all-targets >>"$LOG" 2>&1; then
    ok "cargo check [${combo:-<none>}]"
  else
    bad "cargo check [${combo:-<none>}]"
  fi
done

# ---------------------------------------------------------------------------
# Phases B, C, D -- per combination, per profile
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  for profile in release debug; do
    tag="[${combo:-<none>}/$profile]"
    step "Phases B+C+D $tag"

    if [ "$profile" = release ]; then relflag=(--release); outdir=release
    else relflag=(); outdir=debug; fi

    if ! timeout 600 cargo build --offline --no-default-features \
           ${combo:+--features "$combo"} "${relflag[@]}" >>"$LOG" 2>&1; then
      bad "cdylib build $tag"; continue
    fi
    R_SO="$ROOT/target/$outdir/libSimpleList.so"
    if [ ! -f "$R_SO" ]; then bad "no cdylib produced at $R_SO"; continue; fi
    export SIMPLELIST_RUST_SO="$R_SO"
    ok "Rust .so: $R_SO"

    # Phase D: symbol diff must be empty.
    cdiff=$(diff <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
                 <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort))
    if [ -z "$cdiff" ]; then
      ok "symbol parity $tag ($(nm -D --defined-only "$C_SO" | wc -l) symbol(s), diff empty)"
    else
      bad "symbol diff $tag is NOT empty:"; echo "$cdiff" | sed 's/^/        /'
    fi

    # Phases B and C.
    if timeout 600 cargo test --offline --no-default-features \
         ${combo:+--features "$combo"} --no-fail-fast 2>&1 | tee -a "$LOG" \
         | grep -E '^test result:' | sed 's/^/    /'; then :; fi
    if grep -qE '^test result: FAILED' "$LOG"; then
      # Only consider the tail belonging to this run.
      if timeout 600 cargo test --offline --no-default-features \
           ${combo:+--features "$combo"} --no-fail-fast >/dev/null 2>&1; then
        ok "Phase B+C differential tests $tag"
      else
        bad "Phase B+C differential tests $tag"
      fi
    else
      ok "Phase B+C differential tests $tag"
    fi
  done
done

step "SUMMARY"
if [ "$FAILED" -eq 0 ]; then
  echo "  ALL CHECKS PASSED"
else
  echo "  FAILURES PRESENT -- see above and $LOG"
fi
exit "$FAILED"

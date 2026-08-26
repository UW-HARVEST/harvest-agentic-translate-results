#!/usr/bin/env bash
# Full verification driver: enumerates every build-time configuration, checks
# each one compiles, rebuilds both shared objects, proves symbol parity, and runs
# the Phase B + Phase C differential suites against every Rust artifact.
#
# Usage: ./run_all_tests.sh
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$PWD"
CARGO="cargo"
OFFLINE="--offline"
FAILED=0

WORK="${TMPDIR:-/tmp}/driver-verify.$$"
mkdir -p "$WORK" || { echo "cannot create work dir $WORK"; exit 1; }
trap 'rm -rf "$WORK"' EXIT

hr()  { printf '\n=============== %s ===============\n' "$1"; }
ok()  { printf '  [PASS] %s\n' "$1"; }
bad() { printf '  [FAIL] %s\n' "$1"; FAILED=1; }

# ---------------------------------------------------------------------------
# Phase A.1 — enumerate every feature combination, mechanically
# ---------------------------------------------------------------------------
hr "FEATURE COMBINATIONS (from Cargo.toml [features])"

FEATURES=$(awk '
  /^\[features\]/ { inf = 1; next }
  /^\[/           { inf = 0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }
' Cargo.toml)

if [ -z "$FEATURES" ]; then
  echo "  Cargo.toml declares NO [features] table."
  echo "  => the complete set of feature combinations is the single EMPTY combination."
  COMBOS=("")
else
  mapfile -t FARR <<< "$FEATURES"
  n=${#FARR[@]}
  echo "  Declared features (${n}): ${FARR[*]}"
  COMBOS=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then combo="${combo:+$combo,}${FARR[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
  echo "  => $(( 1 << n )) combinations to verify."
fi

# ---------------------------------------------------------------------------
# Phase A.2 — cargo check every combination
# ---------------------------------------------------------------------------
hr "cargo check, EVERY feature combination"
for combo in "${COMBOS[@]}"; do
  label="--no-default-features${combo:+ --features $combo}"
  if timeout 600 $CARGO check $OFFLINE --no-default-features \
       ${combo:+--features "$combo"} --all-targets > "$WORK/chk.log" 2>&1; then
    ok "cargo check $label"
  else
    bad "cargo check $label"; tail -30 "$WORK/chk.log"
  fi
done
for extra in "--all-features" ""; do
  if timeout 600 $CARGO check $OFFLINE $extra --all-targets > "$WORK/chk.log" 2>&1; then
    ok "cargo check ${extra:-<default features>}"
  else
    bad "cargo check ${extra:-<default features>}"; tail -30 "$WORK/chk.log"
  fi
done

# ---------------------------------------------------------------------------
# Build the C shared library
# ---------------------------------------------------------------------------
hr "BUILD C SHARED LIBRARY"
mkdir -p c_src/build
if (cd c_src/build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > "$WORK/cmake.log" 2>&1 \
      && cmake --build . >> "$WORK/cmake.log" 2>&1); then
  ok "c_src/build/libdriver.so"
else
  bad "C build"; tail -30 "$WORK/cmake.log"; exit 1
fi
C_SO="$ROOT/c_src/build/libdriver.so"

# ---------------------------------------------------------------------------
# Phase D — symbol parity
# ---------------------------------------------------------------------------
hr "PHASE D: SYMBOL PARITY (nm -D)"
nm -D --defined-only "$C_SO" | awk '{print $3}' | grep -v '^$' | sort -u > "$WORK/c_syms"
C_SYM_COUNT=$(wc -l < "$WORK/c_syms")
if [ "$C_SYM_COUNT" -lt 1 ]; then
  bad "no symbols extracted from the C .so — nm/awk pipeline is broken"; exit 1
fi
echo "  C .so exports $C_SYM_COUNT symbol(s): $(tr '\n' ' ' < "$WORK/c_syms")"

symbol_parity() {
  local rs_so="$1" label="$2"
  if [ ! -f "$rs_so" ]; then bad "$label: $rs_so does not exist"; return; fi
  nm -D --defined-only "$rs_so" | awk '{print $3}' | grep -v '^$' | sort -u > "$WORK/rs_syms"
  local missing extra undef
  missing=$(comm -23 "$WORK/c_syms" "$WORK/rs_syms")
  extra=$(comm -13 "$WORK/c_syms" "$WORK/rs_syms")
  if [ -z "$missing" ]; then
    ok "$label: 0 missing symbols (all $C_SYM_COUNT C symbols exported by Rust)"
  else
    bad "$label: symbols MISSING from Rust .so:"; echo "$missing" | sed 's/^/      /'
  fi
  if [ -n "$extra" ]; then
    printf '  [info] %s: extra exported symbols: %s\n' "$label" "$(echo $extra)"
  fi
  undef=$(ldd -r "$rs_so" 2>&1 | grep -i 'not found\|undefined symbol')
  if [ -z "$undef" ]; then
    ok "$label: 0 unresolvable undefined symbols"
  else
    bad "$label: unresolvable symbols:"; echo "$undef" | sed 's/^/      /'
  fi
}

# Total tests across the three integration targets:
#   smoke.rs = 3, valid_paths.rs = 37 (CONFIGS.md rows), error_paths.rs = 16
#   (ERRORS.md rows E1-E15 + the generic invalid-pointer matrix).
# Requiring the exact count means a silently skipped or deleted row fails the gate.
EXPECTED_TESTS=56

# Runs the suite and judges it by cargo's REAL exit status and the parsed
# per-target summaries — never by a substring that a passing run could contain.
run_suite() {
  local label="$1"; shift
  timeout 600 env "$@" > "$WORK/t.log" 2>&1
  local rc=$?
  local okc failc passed
  read -r okc failc passed <<< "$(awk '
    /^test result: ok\./   { ok++;   passed += $4 }
    /^test result: FAILED/ { fail++; passed += $4 }
    END { printf "%d %d %d", ok+0, fail+0, passed+0 }' "$WORK/t.log")"

  # NOTE: do not grep for "panicked" here. When the DEBUG artifact is under test,
  # rustc's debug_assertions UB check makes the *forked child* panic on the
  # null-pointer rows and that message lands in this log by design. The
  # authoritative signals are cargo's exit status and libtest's own summaries.
  if [ "$rc" -eq 0 ] && [ "$failc" -eq 0 ] && [ "$passed" -eq "$EXPECTED_TESTS" ] \
     && ! grep -q '^error: test failed' "$WORK/t.log"; then
    ok "$label — $passed/$EXPECTED_TESTS tests passed across $okc targets"
  else
    bad "$label (exit=$rc, failed-targets=$failc, passed=$passed/$EXPECTED_TESTS)"
    grep -E '^test result|panicked|^error' "$WORK/t.log" | head -20
    tail -25 "$WORK/t.log"
  fi
}

# ---------------------------------------------------------------------------
# Phases B + C for every combination x every profile
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  clabel="${combo:-<no features>}"
  featflag=(--no-default-features)
  [ -n "$combo" ] && featflag+=(--features "$combo")

  hr "COMBO: $clabel — build both profiles"
  if timeout 600 $CARGO build $OFFLINE "${featflag[@]}" --release > "$WORK/b.log" 2>&1
  then ok "release cdylib"; else bad "release cdylib build"; tail -20 "$WORK/b.log"; fi
  if timeout 600 $CARGO build $OFFLINE "${featflag[@]}" > "$WORK/b.log" 2>&1
  then ok "debug cdylib";   else bad "debug cdylib build";   tail -20 "$WORK/b.log"; fi

  symbol_parity "$ROOT/target/release/libdriver.so" "combo=$clabel release"
  symbol_parity "$ROOT/target/debug/libdriver.so"   "combo=$clabel debug"

  # --- release artifact: the deliverable; full parity including fault signals.
  hr "COMBO: $clabel — PHASES B+C against RELEASE cdylib"
  run_suite "combo=$clabel RELEASE differential suite" \
    RUST_DRIVER_SO="$ROOT/target/release/libdriver.so" \
    $CARGO test $OFFLINE "${featflag[@]}"

  # --- debug artifact: same suite. rustc's debug_assertions UB checks turn a
  #     null deref into SIGABRT instead of SIGSEGV; the harness tolerates that
  #     ONLY when DRIVER_RUST_UB_CHECKS=1 (see Harness::assert_fault_parity).
  hr "COMBO: $clabel — PHASES B+C against DEBUG cdylib (UB checks on)"
  run_suite "combo=$clabel DEBUG differential suite" \
    RUST_DRIVER_SO="$ROOT/target/debug/libdriver.so" DRIVER_RUST_UB_CHECKS=1 \
    $CARGO test $OFFLINE "${featflag[@]}"
done

hr "RESULT"
if [ "$FAILED" -eq 0 ]; then
  echo "  ALL CONFIGURATIONS PASSED"
else
  echo "  FAILURES PRESENT — see [FAIL] lines above"
fi
exit "$FAILED"

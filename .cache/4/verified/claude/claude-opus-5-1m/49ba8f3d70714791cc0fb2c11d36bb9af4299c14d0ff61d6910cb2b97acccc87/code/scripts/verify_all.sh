#!/usr/bin/env bash
# Phase D driver: enumerate every build configuration and run the full
# Phase B + Phase C differential suite under each one.
#
#   ./scripts/verify_all.sh
#
# Configurations come from two places:
#   * Cargo.toml [features]      -- enumerated mechanically below
#   * c_src/CMakeLists.txt       -- no option()/add_definitions() => 1 config
# plus the two cargo profiles, since `panic = "abort"` and optimisation make the
# release build a genuinely different artifact.

set -uo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD
LOG_DIR=${TMPDIR:-/tmp}/verify_all.$$
mkdir -p "$LOG_DIR"

fail=0
step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [PASS] %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; fail=1; }

run() { # run <logname> <description> <cmd...>
  local log="$LOG_DIR/$1.log"; shift
  local desc="$1"; shift
  if timeout 560 "$@" >"$log" 2>&1; then
    ok "$desc"
  else
    bad "$desc  (log: $log)"
    tail -n 25 "$log" | sed 's/^/      /'
  fi
}

# ---------------------------------------------------------------------------
# 1. Enumerate the feature combinations mechanically
# ---------------------------------------------------------------------------
step "Enumerating build configurations"

FEATURES=$(python3 - <<'PY'
import re, sys
txt = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.S | re.M)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n and n != 'default':
                names.append(n)
print(' '.join(names))
PY
)

if [ -z "${FEATURES// }" ]; then
  echo "Cargo.toml declares no [features] -> exactly one feature combination (empty)"
  COMBOS=("")
else
  # full power set
  read -r -a FARR <<<"$FEATURES"
  COMBOS=()
  n=${#FARR[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then combo="${combo:+$combo,}${FARR[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
  echo "features: $FEATURES  ->  ${#COMBOS[@]} combinations"
fi

echo "CMake options in c_src/CMakeLists.txt:"
if grep -qE '^\s*(option|add_definitions|target_compile_definitions)\(' c_src/CMakeLists.txt; then
  grep -nE '^\s*(option|add_definitions|target_compile_definitions)\(' c_src/CMakeLists.txt
else
  echo "  (none) -> 1 C build configuration"
fi

# ---------------------------------------------------------------------------
# 2. Build the C ground truth exactly as documented
# ---------------------------------------------------------------------------
step "Building the C reference"
mkdir -p c_src/build
run cmake_cfg "cmake configure" \
  cmake -S "$ROOT/c_src" -B "$ROOT/c_src/build" -DCMAKE_POSITION_INDEPENDENT_CODE=ON
run cmake_build "cmake --build" cmake --build "$ROOT/c_src/build"

# ---------------------------------------------------------------------------
# 3. cargo check for every feature combination
# ---------------------------------------------------------------------------
step "cargo check for every feature combination"
i=0
for combo in "${COMBOS[@]}"; do
  i=$((i + 1))
  if [ -z "$combo" ]; then
    run "check_$i" "cargo check --no-default-features" \
      cargo check --offline --no-default-features --all-targets
  else
    run "check_$i" "cargo check --no-default-features --features $combo" \
      cargo check --offline --no-default-features --features "$combo" --all-targets
  fi
done
run check_all_features "cargo check --all-features" \
  cargo check --offline --all-features --all-targets

# ---------------------------------------------------------------------------
# 4. Phases B + C + D for every feature combination, in both profiles
# ---------------------------------------------------------------------------
step "Phase B/C/D differential suite for every configuration"
i=0
for combo in "${COMBOS[@]}"; do
  i=$((i + 1))
  for profile in dev release; do
    relflag=""; [ "$profile" = release ] && relflag="--release"
    featflag=(--no-default-features)
    [ -n "$combo" ] && featflag+=(--features "$combo")
    label="features='${combo:-<none>}' profile=$profile"

    # Build both artifacts for this profile up front: an integration test cannot
    # link a cdylib-only lib, so cargo would not build libdriver.so by itself.
    run "build_${i}_$profile" "cargo build (bin + cdylib) [$label]" \
      cargo build --offline $relflag "${featflag[@]}" --lib --bins

    run "test_cli_${i}_$profile" "differential_cli  [$label]" \
      cargo test --offline $relflag "${featflag[@]}" --test differential_cli -- --test-threads=4
    run "test_so_${i}_$profile" "differential_so   [$label]" \
      cargo test --offline $relflag "${featflag[@]}" --test differential_so -- --test-threads=1
    run "test_sym_${i}_$profile" "symbol_parity     [$label]" \
      cargo test --offline $relflag "${featflag[@]}" --test symbol_parity
  done
done

# ---------------------------------------------------------------------------
# 5. The symbol diff, printed for the record
# ---------------------------------------------------------------------------
step "nm -D symbol diff (must be empty)"
CSO="$LOG_DIR/libcdriver.so"
cc -shared -fPIC -O2 -o "$CSO" c_src/src/main.c
for so in target/debug/libdriver.so target/release/libdriver.so; do
  [ -f "$so" ] || continue
  nm -D --defined-only "$CSO" | grep -vE ' [wWvV] ' | awk '{print $NF}' | sort >"$LOG_DIR/c.syms"
  nm -D --defined-only "$so" | grep -vE ' [wWvV] ' | awk '{print $NF}' | sort >"$LOG_DIR/r.syms"
  if diff_out=$(comm -23 "$LOG_DIR/c.syms" "$LOG_DIR/r.syms"); [ -z "$diff_out" ]; then
    ok "$so exports every C symbol ($(wc -l <"$LOG_DIR/c.syms") total)"
  else
    bad "$so is missing: $diff_out"
  fi
done

step "RESULT"
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASS"
else
  echo "FAILURES PRESENT (logs in $LOG_DIR)"
fi
exit "$fail"

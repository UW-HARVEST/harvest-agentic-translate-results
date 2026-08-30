#!/usr/bin/env bash
# Full verification sweep: build both libraries, diff their exported symbols,
# and run the differential suite under EVERY cargo feature combination.
#
# Usage:  cd translation && ./scripts/verify.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
cd "$HERE"

fail=0
step() { printf '\n=== %s ===\n' "$*"; }

# --------------------------------------------------------------------------
step "1. build the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
ls -l "$C_SO"

# --------------------------------------------------------------------------
step "2. build the Rust cdylib"
cargo build --release >/dev/null 2>&1 || { echo "Rust build FAILED"; exit 1; }
R_SO="$HERE/target/release/libdriver.so"
ls -l "$R_SO"

# --------------------------------------------------------------------------
step "3. symbol parity (nm -D --defined-only)"
syms() { nm -D --defined-only "$1" | awk '$(NF-1) ~ /^[A-Z]$/ {print $NF}' | sort -u; }
CS=$(syms "$C_SO"); RS=$(syms "$R_SO")
echo "C   exports: $(echo "$CS" | tr '\n' ' ')"
echo "Rust exports: $(echo "$RS" | tr '\n' ' ')"
MISSING=$(comm -23 <(echo "$CS") <(echo "$RS"))
EXTRA=$(comm -13 <(echo "$CS") <(echo "$RS"))
if [ -n "$MISSING" ]; then echo "MISSING FROM RUST: $MISSING"; fail=1; else echo "missing from Rust: (none)"; fi
if [ -n "$EXTRA" ];   then echo "extra in Rust:     $EXTRA";   else echo "extra in Rust:     (none)"; fi

# --------------------------------------------------------------------------
step "4. enumerate cargo feature combinations"
FEATURES=$(cargo metadata --no-deps --format-version 1 --offline 2>/dev/null \
  | python3 -c 'import json,sys; m=json.load(sys.stdin); print(" ".join(sorted(f for f in m["packages"][0]["features"] if f!="default")))')
if [ -z "${FEATURES// }" ]; then
  echo "this crate declares NO cargo features; the only configurations are:"
  echo "  - default              (no features)"
  echo "  - --no-default-features (identical: the default feature set is empty)"
  COMBOS=("default" "no-default")
else
  echo "features: $FEATURES"
  # powerset of the declared features, with and without default features
  COMBOS=($(python3 - "$FEATURES" <<'PY'
import itertools, sys
fs = sys.argv[1].split()
out = ["default", "no-default"]
for r in range(1, len(fs) + 1):
    for c in itertools.combinations(fs, r):
        out.append("feat:" + ",".join(c))
        out.append("nodefault-feat:" + ",".join(c))
print(" ".join(out))
PY
))
fi

# --------------------------------------------------------------------------
step "5. run the differential suite for each combination"
for combo in "${COMBOS[@]}"; do
  args=()
  unset DRIVER_TEST_FEATURES DRIVER_TEST_NO_DEFAULT_FEATURES
  case "$combo" in
    default)              ;;
    no-default)           args+=(--no-default-features); export DRIVER_TEST_NO_DEFAULT_FEATURES=1 ;;
    feat:*)               f="${combo#feat:}";            args+=(--features "$f"); export DRIVER_TEST_FEATURES="$f" ;;
    nodefault-feat:*)     f="${combo#nodefault-feat:}";  args+=(--no-default-features --features "$f")
                          export DRIVER_TEST_NO_DEFAULT_FEATURES=1 DRIVER_TEST_FEATURES="$f" ;;
  esac
  printf -- '--- combo: %s   (cargo test %s) ---\n' "$combo" "${args[*]:-<none>}"
  if timeout 600 cargo test --no-fail-fast "${args[@]}" 2>&1 | grep -E 'test result:|FAILED|^error'; then :; fi
  if timeout 600 cargo test --no-fail-fast "${args[@]}" >/dev/null 2>&1; then
    echo "    PASS"
  else
    echo "    FAIL"; fail=1
  fi
done

# --------------------------------------------------------------------------
step "6. cargo check under each combination"
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    default)          a=() ;;
    no-default)       a=(--no-default-features) ;;
    feat:*)           a=(--features "${combo#feat:}") ;;
    nodefault-feat:*) a=(--no-default-features --features "${combo#nodefault-feat:}") ;;
  esac
  if cargo check --tests "${a[@]}" >/dev/null 2>&1; then echo "  check $combo: ok"; else echo "  check $combo: FAILED"; fail=1; fi
done

step "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "SOME CHECKS FAILED"; fi
exit "$fail"

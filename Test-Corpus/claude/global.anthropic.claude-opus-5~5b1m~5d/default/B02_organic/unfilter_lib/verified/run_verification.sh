#!/usr/bin/env bash
# Full verification run: builds both C configurations and the Rust cdylib, then
# runs the whole differential test suite under EVERY feature combination the
# crate declares.
#
#   ./run_verification.sh
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
fail=0

step() { printf '\n=== %s ===\n' "$*"; }

# --------------------------------------------------------------------------
step "1. C reference, task's build command (asserts LIVE, no NDEBUG)"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) >/dev/null || { echo "C build failed"; exit 1; }
C_ASSERT=$(ls "$ROOT"/c_src/build/*.so | head -1)
echo "  $C_ASSERT"

step "2. C reference, Release (NDEBUG, no asserts)"
cmake -S "$ROOT/c_src" -B "$ROOT/cbuild_release" \
      -DCMAKE_BUILD_TYPE=Release -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build "$ROOT/cbuild_release" >/dev/null \
  || { echo "C release build failed"; exit 1; }
C_RELEASE=$(ls "$ROOT"/cbuild_release/*.so | head -1)
echo "  $C_RELEASE"

# --------------------------------------------------------------------------
# Every feature combination declared in Cargo.toml.  The crate declares no
# [features] table at all, so the combinations collapse to the default; the
# loop is written generically so it keeps working if features are ever added.
step "3. Enumerating feature combinations"
FEATURES=$(python3 - <<'PY'
import re, sys
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n != 'default':
                names.append(n)
print(' '.join(names))
PY
)
if [ -z "$FEATURES" ]; then
  echo "  crate declares no [features]; the only configurations are:"
  COMBOS=("" "--no-default-features" "--all-features")
else
  echo "  features: $FEATURES"
  COMBOS=("" "--no-default-features" "--all-features")
  # powerset of the declared features, with --no-default-features
  n=0
  for f in $FEATURES; do n=$((n+1)); done
  total=$((1 << n))
  for ((mask=0; mask<total; mask++)); do
    sel=(); i=0
    for f in $FEATURES; do
      if (( (mask >> i) & 1 )); then sel+=("$f"); fi
      i=$((i+1))
    done
    if [ ${#sel[@]} -gt 0 ]; then
      COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
fi
printf '  %s\n' "${COMBOS[@]/#/combo: }"

# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  step "4. cargo build + test  [${combo:-default}]"
  # shellcheck disable=SC2086
  cargo build --release --offline $combo >/dev/null 2>&1 \
    || { echo "  BUILD FAILED"; fail=1; continue; }
  RUST_SO="$PWD/target/release/libunfilter_lib.so"
  echo "  Rust .so: $RUST_SO"

  # symbol parity, printed for the record
  diff <(nm -D --defined-only "$C_ASSERT" | awk '{print $3}' | sort) \
       <(nm -D --defined-only "$RUST_SO"  | awk '{print $3}' | sort) \
    && echo "  symbol diff: EMPTY" \
    || { echo "  SYMBOL DIFF NOT EMPTY"; fail=1; }

  # shellcheck disable=SC2086
  C_SO_ASSERTS="$C_ASSERT" C_SO_RELEASE="$C_RELEASE" RUST_SO="$RUST_SO" \
    cargo test --release --offline $combo -- --test-threads=1 2>&1 \
    | grep -E '^test |test result|FAILED|panicked' \
    || true
  # shellcheck disable=SC2086
  C_SO_ASSERTS="$C_ASSERT" C_SO_RELEASE="$C_RELEASE" RUST_SO="$RUST_SO" \
    cargo test --release --offline $combo -- --test-threads=1 >/dev/null 2>&1 \
    || { echo "  TESTS FAILED"; fail=1; }
done

step "RESULT"
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASS"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"

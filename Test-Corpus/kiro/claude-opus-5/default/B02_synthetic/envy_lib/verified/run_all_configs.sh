#!/usr/bin/env bash
# Runs the full differential suite across every configuration axis that exists
# in this project:
#   * every Cargo feature combination (enumerated from Cargo.toml)
#   * both Rust build profiles (debug = opt-level 0, release = opt-level 3 +
#     panic=abort), since the .so is the shipped artifact in the release case
#   * the sanctioned C build, plus optimized C builds (informational: the C's
#     signed-overflow UB can be compiled differently at -O2/-O3)
set -u
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"

FEATURES=$(python3 - <<'PY'
import re
s = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        k = line.split('=')[0].strip()
        if k and k != 'default':
            names.append(k)
print(' '.join(names))
PY
)
echo "declared features: [${FEATURES:-none}]"

# Build every feature combination (powerset). With no features this is just the
# default build plus --no-default-features.
declare -a COMBOS=()
if [ -z "$FEATURES" ]; then
  COMBOS+=("")                       # default
  COMBOS+=("--no-default-features")
else
  read -ra FARR <<<"$FEATURES"
  n=${#FARR[@]}
  for ((mask=0; mask<(1<<n); mask++)); do
    sel=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then sel="$sel,${FARR[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features ${sel#,}")
  done
  COMBOS+=("")                       # plus the default feature set
fi

fail=0

echo "=== cargo check across feature combinations ==="
for c in "${COMBOS[@]}"; do
  if timeout 600 cargo check $c >/tmp/fc.log 2>&1; then
    echo "  ok    : cargo check ${c:-<default>}"
  else
    echo "  FAILED: cargo check ${c:-<default>}"; tail -20 /tmp/fc.log; fail=1
  fi
done

# ---------------------------------------------------------------------------
# C library variants. The sanctioned build (no CMAKE_BUILD_TYPE) is the ground
# truth; the optimized builds are extra configurations of the same source.
# Built out of tree so nothing under c_src/ is touched beyond c_src/build.
# ---------------------------------------------------------------------------
declare -a C_SOS=()
declare -a C_TAGS=()
C_SOS+=("$(ls "$ROOT"/c_src/build/*.so)"); C_TAGS+=("c:sanctioned(no -O)")
for bt in O2 O3; do
  d=/tmp/c_build_$bt
  rm -rf "$d"; mkdir -p "$d"
  if cmake -S "$ROOT/c_src" -B "$d" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DCMAKE_C_FLAGS="-$bt" >/dev/null 2>&1 && cmake --build "$d" >/dev/null 2>&1; then
    C_SOS+=("$(ls "$d"/*.so)"); C_TAGS+=("c:-$bt")
  else
    echo "  note: could not build the C library with -$bt, skipping"
  fi
done

echo "=== full differential suite across configurations ==="
for c in "${COMBOS[@]}"; do
  for profile in debug release; do
    if [ "$profile" = release ]; then relflag="--release"; else relflag=""; fi
    if ! timeout 600 cargo build $relflag $c >/tmp/fb.log 2>&1; then
      echo "  FAILED: cargo build $profile ${c:-<default>}"; tail -20 /tmp/fb.log; fail=1; continue
    fi
    RSO="target/$profile/libenvy_lib.so"
    if [ ! -f "$RSO" ]; then echo "  FAILED: $RSO not produced"; fail=1; continue; fi
    for i in "${!C_SOS[@]}"; do
      tag="rust:$profile ${c:-<default>} vs ${C_TAGS[$i]}"
      out=$(DIFFTEST_C_SO="${C_SOS[$i]}" DIFFTEST_RUST_SO="$(pwd)/$RSO" \
            timeout 600 cargo test $relflag $c -- --test-threads=1 2>&1)
      passed=$(printf '%s' "$out" | grep -cE '^test result: ok\.')
      failed=$(printf '%s' "$out" | grep -cE '^test result: FAILED')
      total=$(printf '%s' "$out" | grep -oE '^test result: ok\. [0-9]+ passed' | grep -oE '[0-9]+' | paste -sd+ - | python3 -c "import sys;print(eval(sys.stdin.read().strip() or 0))")
      if [ "$failed" -eq 0 ]; then
        echo "  ok    : $tag  (${total:-0} tests, $passed binaries)"
      else
        echo "  FAILED: $tag"
        printf '%s' "$out" | grep -E '^test .* FAILED|DIVERGENCE' | head -10
        # Only the sanctioned C build is a correctness gate.
        case "${C_TAGS[$i]}" in
          c:sanctioned*) fail=1 ;;
          *) echo "        (non-gating: optimized C build of code with signed-overflow UB)" ;;
        esac
      fi
    done
  done
done

echo "----"
if [ $fail -eq 0 ]; then echo "ALL CONFIGURATIONS PASS"; else echo "FAILURES PRESENT"; fi
exit $fail

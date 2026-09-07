#!/usr/bin/env bash
# Enumerate every feature combination declared in Cargo.toml and run the full
# differential suite for each, in both the dev and release profiles, against a
# freshly built C shared library.
#
# Usage:  ./check_features.sh
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
CARGO="cargo --offline"
FAIL=0

echo "=== building the C shared library ==="
(
  cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null
) || { echo "FATAL: C build failed"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
# Enumerate features from Cargo.toml. This crate declares none, so the set of
# combinations is exactly {default} == {no-default-features}; the loop below is
# written generically so it stays correct if features are ever added.
# ---------------------------------------------------------------------------
FEATURES=$(python3 - <<'PY'
import re, sys
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
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
echo "declared features: [${FEATURES:-<none>}]"

# Build the combination list: always the default build and the
# --no-default-features build, then every subset of the declared features.
COMBOS=()
COMBOS+=("--")                      # default
COMBOS+=("--no-default-features")   # empty feature set
if [ -n "$FEATURES" ]; then
  read -r -a FARR <<<"$FEATURES"
  n=${#FARR[@]}
  total=$((1 << n))
  for ((mask = 1; mask < total; mask++)); do
    set=""
    for ((b = 0; b < n; b++)); do
      if (( (mask >> b) & 1 )); then set="$set,${FARR[b]}"; fi
    done
    COMBOS+=("--no-default-features --features ${set#,}")
  done
fi

echo "=== ${#COMBOS[@]} feature combination(s) x 2 profiles ==="

TMP_OUT="$(mktemp)"
trap 'rm -f "$TMP_OUT"' EXIT
run_one() {
  local profile_flag="$1" profile_dir="$2" combo="$3"
  local label="profile=${profile_dir} combo=[${combo}]"
  local cflags=()
  [ "$combo" != "--" ] && read -r -a cflags <<<"$combo"

  if ! $CARGO build $profile_flag "${cflags[@]}" >/dev/null 2>&1; then
    echo "FAIL(build) $label"; FAIL=1; return
  fi
  local rso="target/${profile_dir}/libgjk_lib.so"
  if [ ! -f "$rso" ]; then echo "FAIL(no .so) $label"; FAIL=1; return; fi

  local missing
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
    <(nm -D --defined-only "$rso" | awk '{print $3}' | sort))
  if [ -n "$missing" ]; then
    echo "FAIL(symbols) $label -- missing: $(echo "$missing" | tr '\n' ' ')"
    FAIL=1; return
  fi

  if GJK_RUST_SO="$(pwd)/$rso" timeout 600 $CARGO test $profile_flag \
      "${cflags[@]}" --no-fail-fast -q >"$TMP_OUT" 2>&1; then
    local nsym
    nsym=$(nm -D --defined-only "$rso" | awk '{print $3}' | wc -l)
    local npass
    npass=$(grep -oE "^test result: ok\. [0-9]+" "$TMP_OUT" \
            | awk '{s+=$4} END{print s+0}')
    echo "PASS $label  (exports=$nsym, tests passed=$npass)"
  else
    echo "FAIL(tests) $label"
    grep -E "^test result|panicked|mismatch" "$TMP_OUT" | head -20
    FAIL=1
  fi
}

for combo in "${COMBOS[@]}"; do
  run_one ""          debug   "$combo"
  run_one "--release" release "$combo"
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$FAIL"

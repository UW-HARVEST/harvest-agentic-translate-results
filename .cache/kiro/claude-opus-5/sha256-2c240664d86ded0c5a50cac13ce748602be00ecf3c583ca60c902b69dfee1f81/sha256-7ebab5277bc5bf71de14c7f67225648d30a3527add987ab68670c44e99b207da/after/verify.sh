#!/usr/bin/env bash
# Build the C reference library and verify the Rust translation against it for
# every build-time configuration.
#
#   * c_src/CMakeLists.txt exposes no options, so the C side has exactly one
#     configuration.
#   * translation/Cargo.toml has no [features] table, so the only valid feature
#     combination is the empty one.
#
# The script still enumerates the combinations programmatically so that adding
# a feature later is picked up automatically.
set -uo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
TIMEOUT=600
fail=0

echo "== building the C reference library =="
mkdir -p "$ROOT/c_src/build"
(
  cd "$ROOT/c_src/build" &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
  cmake --build . >/dev/null
) || { echo "C build FAILED"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -name '*.so' | head -1)"
echo "   $C_SO"

echo "== regenerating fixtures =="
python3 "$ROOT/gen_fixtures.py" || { echo "fixture generation FAILED"; exit 1; }

# ---------------------------------------------------------------------------
# enumerate feature combinations
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(python3 - "$ROOT/translation/Cargo.toml" <<'PY'
import sys, re
txt = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=')[0].strip()
        if name not in ('default',):
            names.append(name)
print('\n'.join(names))
PY
)

COMBOS=("")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  COMBOS=()
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "== feature combinations: ${#COMBOS[@]} =="
for c in "${COMBOS[@]}"; do echo "   '${c:-<none>}'"; done

cd "$ROOT/translation"

for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  args=(--no-default-features)
  [ -n "$combo" ] && args+=(--features "$combo")

  echo
  echo "===================================================================="
  echo " combination: $label"
  echo "===================================================================="

  echo "-- cargo check"
  if ! timeout $TIMEOUT cargo check "${args[@]}" --all-targets 2>&1 | tail -20; then
    echo "CHECK FAILED for $label"; fail=1; continue
  fi

  # Both profiles are exercised: `release` is the artifact an external consumer
  # links against (panic=abort, no overflow checks); `dev` additionally turns on
  # debug assertions and integer-overflow checks, which catches any arithmetic
  # that only accidentally matches C's wrapping behaviour.
  for profile in release dev; do
    echo "-- cargo build ($profile)"
    build=("${args[@]}")
    [ "$profile" = release ] && build+=(--release)
    if ! timeout $TIMEOUT cargo build "${build[@]}" 2>&1 | tail -20; then
      echo "BUILD FAILED for $label/$profile"; fail=1; continue
    fi
    dir=release; [ "$profile" = dev ] && dir=debug
    RS_SO="$ROOT/translation/target/$dir/libload_png_mem_lib.so"

    echo "-- nm symbol comparison ($profile)"
    missing="$(comm -23 \
      <(nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort -u) \
      <(nm -D --defined-only "$RS_SO" | awk '{print $3}' | sort -u))"
    if [ -n "$missing" ]; then
      echo "MISSING SYMBOLS in $RS_SO:"; echo "$missing"; fail=1
    else
      echo "   all C symbols exported"
    fi

    echo "-- cargo test ($profile, against $dir .so)"
    if ! C_SO="$C_SO" RUST_SO="$RS_SO" timeout $TIMEOUT \
        cargo test "${args[@]}" --release -- --test-threads=1 2>&1 \
          | grep -E 'Running|test result|FAILED|panicked|compared|skipped'; then
      echo "TESTS FAILED for $label/$profile"; fail=1
    fi
  done
done

echo
if [ "$fail" = 0 ]; then
  echo "ALL CONFIGURATIONS PASS"
else
  echo "FAILURES DETECTED"
fi
exit $fail

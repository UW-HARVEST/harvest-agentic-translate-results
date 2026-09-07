#!/usr/bin/env bash
# Full verification run: build both libraries, then run the differential suite
# under every feature combination declared in Cargo.toml, against both the
# default (-O0) and an optimised (-O2) build of the C reference.
set -euo pipefail
cd "$(dirname "$0")"
WS=$(cd .. && pwd)
CARGO_FLAGS=${CARGO_FLAGS:---offline}

echo "=== building the C reference (default flags) ==="
mkdir -p "$WS/c_src/build"
(cd "$WS/c_src/build" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null)
C_DEFAULT=$(ls "$WS"/c_src/build/*.so | head -1)

echo "=== building the C reference (-O2) ==="
mkdir -p target/cbuild_o2
(cd target/cbuild_o2 && cmake "$WS/c_src" -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DCMAKE_BUILD_TYPE=Release >/dev/null && cmake --build . >/dev/null)
C_O2=$(ls target/cbuild_o2/*.so | head -1)

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml.
# ---------------------------------------------------------------------------
FEATURES=$(python3 - <<'PY'
import re, pathlib
txt = pathlib.Path("Cargo.toml").read_text()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
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

declare -a COMBOS=()
if [ -z "$FEATURES" ]; then
  # No [features] section: the default build and --no-default-features are the
  # only two configurations, and they are identical. Verify both anyway.
  COMBOS+=("")
  COMBOS+=("--no-default-features")
else
  set -- $FEATURES
  n=$#
  # power set of the declared features
  for ((mask=0; mask<(1<<n); mask++)); do
    combo=""
    i=0
    for f in $FEATURES; do
      if (( (mask >> i) & 1 )); then combo="${combo:+$combo,}$f"; fi
      i=$((i+1))
    done
    COMBOS+=("--no-default-features${combo:+ --features $combo}")
  done
  COMBOS+=("")  # the default feature set
fi

fail=0
for combo in "${COMBOS[@]}"; do
  label=${combo:-"(default features)"}
  echo
  echo "############################################################"
  echo "# features: $label"
  echo "############################################################"
  # shellcheck disable=SC2086
  cargo build --release $CARGO_FLAGS $combo
  for c_so in "$C_DEFAULT" "$C_O2"; do
    echo "--- C reference: $c_so"
    # shellcheck disable=SC2086
    if ! C_SO="$c_so" RUST_SO="$PWD/target/release/libreverse_collide_lib.so" \
         cargo test --release $CARGO_FLAGS $combo -- --test-threads="$(nproc)"; then
      fail=1
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES DETECTED"
fi
exit "$fail"

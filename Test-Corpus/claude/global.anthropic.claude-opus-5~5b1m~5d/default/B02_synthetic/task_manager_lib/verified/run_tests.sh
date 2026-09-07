#!/usr/bin/env bash
# Builds the C shared library and the Rust cdylib, then runs the differential
# test-suite across every feature combination.
#
#   ./run_tests.sh            # debug cdylib
#   ./run_tests.sh --release  # release cdylib
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
CARGO_FLAGS=(--offline)
rc=0

# ---------------------------------------------------------------- C library
echo "== building C shared library =="
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
ls -l "$root/c_src/build/libdriver.so"

# ------------------------------------------------- feature combinations
# Cargo.toml has no [features] table, so the only combination is the default
# one; `--no-default-features` is equivalent. Both are exercised anyway so the
# matrix stays correct if features are ever added.
mapfile -t FEATURES < <(
  python3 - "$here/Cargo.toml" <<'PY'
import itertools, sys, re
txt = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n != 'default':
                names.append(n)
if not names:
    print('__default__')
    print('__none__')
else:
    print('__default__')
    for r in range(len(names) + 1):
        for combo in itertools.combinations(names, r):
            print(','.join(combo) if combo else '__none__')
PY
)

for combo in "${FEATURES[@]}"; do
  case "$combo" in
    __default__) label="default";        FLAGS=() ;;
    __none__)    label="no-default";     FLAGS=(--no-default-features) ;;
    *)           label="no-default+$combo"; FLAGS=(--no-default-features --features "$combo") ;;
  esac

  for prof in debug release; do
    if [[ $prof == release ]]; then PROF=(--release); else PROF=(); fi
    echo
    echo "===================================================================="
    echo "== features: $label   profile: $prof"
    echo "===================================================================="
    ( cd "$here" && cargo build "${CARGO_FLAGS[@]}" "${FLAGS[@]}" "${PROF[@]}" ) \
      || { echo "cargo build FAILED ($label/$prof)"; rc=1; continue; }
    ( cd "$here" && DRIVER_RUST_SO="$here/target/$prof/libdriver.so" \
        timeout 600 cargo test "${CARGO_FLAGS[@]}" "${FLAGS[@]}" "${PROF[@]}" \
        -- --test-threads=1 ) \
      || { echo "TESTS FAILED ($label/$prof)"; rc=1; }
  done
done

echo
if [[ $rc -eq 0 ]]; then echo "ALL COMBINATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit $rc

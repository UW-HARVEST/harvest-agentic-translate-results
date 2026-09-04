#!/usr/bin/env bash
# Phase D driver: run the full differential suite against every Rust artifact
# and every feature combination, then diff the exported symbols.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
C_SO="$ROOT/c_src/build/libdriver.so"
fail=0

echo "== building the C shared library =="
(cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
test -f "$C_SO" || { echo "missing $C_SO"; exit 1; }

# Feature combinations: derived from Cargo.toml. This crate declares no
# [features] table, so the only combinations are the default / empty ones.
mapfile -t FEATURES < <(python3 - <<'PY'
import re
src = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.S | re.M)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.strip()
        if line and not line.startswith('#') and '=' in line:
            names.append(line.split('=')[0].strip())
import itertools
combos = ['<default>', '<none>']
for r in range(1, len(names) + 1):
    for c in itertools.combinations(names, r):
        combos.append(','.join(c))
print('\n'.join(combos))
PY
)
echo "== feature combinations: ${FEATURES[*]} =="

for combo in "${FEATURES[@]}"; do
  case "$combo" in
    '<default>') FEATFLAGS=() ;;
    '<none>')    FEATFLAGS=(--no-default-features) ;;
    *)           FEATFLAGS=(--no-default-features --features "$combo") ;;
  esac

  echo
  echo "########## features: $combo ##########"
  timeout 600 cargo check --all-targets "${FEATFLAGS[@]}" >/dev/null 2>&1 \
    || { echo "cargo check FAILED ($combo)"; fail=1; continue; }

  for profile in dev release; do
    if [ "$profile" = release ]; then
      PROFFLAGS=(--release); OUT=target/release
    else
      PROFFLAGS=();          OUT=target/debug
    fi

    timeout 600 cargo build "${PROFFLAGS[@]}" "${FEATFLAGS[@]}" >/dev/null 2>&1 \
      || { echo "cargo build FAILED ($combo/$profile)"; fail=1; continue; }

    RUST_SO="$OUT/libdriver.so"
    echo "---- symbol diff: $RUST_SO ----"
    if diff <(nm -D --defined-only "$C_SO"    | awk '{print $3}' | sort) \
            <(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort); then
      echo "symbol sets identical ($(nm -D --defined-only "$C_SO" | wc -l) symbols)"
    else
      echo "SYMBOL DIFF NON-EMPTY ($combo/$profile)"; fail=1
    fi

    echo "---- tests: features=$combo profile=$profile so=$RUST_SO ----"
    DRIVER_C_SO="$C_SO" DRIVER_RUST_SO="$(pwd)/$RUST_SO" \
      timeout 600 cargo test "${FEATFLAGS[@]}" 2>&1 \
      | grep -E '^(test result|running|error|warning: unused|---- )' \
      | sed 's/^/    /'
    rc=${PIPESTATUS[1]}
    if [ "$rc" != 0 ]; then
      echo "TESTS FAILED ($combo/$profile) rc=$rc"; fail=1
    fi
  done
done

echo
if [ "$fail" = 0 ]; then echo "ALL GREEN"; else echo "FAILURES PRESENT"; fi
exit "$fail"

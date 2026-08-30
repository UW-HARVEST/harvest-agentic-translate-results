#!/usr/bin/env bash
# Build the C reference library and run the differential test suite against the
# Rust cdylib for every build configuration.
#
# `translation/Cargo.toml` declares no `[features]` and `c_src/CMakeLists.txt`
# declares no options or compile definitions, so the only build-time axis left
# is the cargo profile. Both profiles are built and tested explicitly.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

echo "== building C reference library =="
(cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)

cd translation

echo "== enumerating cargo feature combinations =="
mapfile -t combos < <(python3 - <<'PY'
import re, itertools
s = open('Cargo.toml').read()
m = re.search(r'^\[features\](.*?)(^\[|\Z)', s, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip()
            if n != 'default':
                names.append(n)
for r in range(len(names) + 1):
    for c in itertools.combinations(names, r):
        print(','.join(c))
PY
)
printf 'combinations: %s\n' "${#combos[@]}"

for combo in "${combos[@]}"; do
  label="${combo:-<no features>}"
  echo "== cargo check --no-default-features --features '${combo}' (${label}) =="
  timeout 600 cargo check --no-default-features --features "$combo"

  for profile in dev release; do
    if [[ "$profile" == release ]]; then
      flag=(--release); dir=release
    else
      flag=(); dir=debug
    fi
    echo "== ${profile} / ${label}: build cdylib + run differential tests =="
    timeout 600 cargo build "${flag[@]}" --no-default-features --features "$combo"
    export DRIVER_RUST_SO="$root/translation/target/$dir/libdriver.so"
    test -f "$DRIVER_RUST_SO" || { echo "missing $DRIVER_RUST_SO"; exit 1; }

    echo "-- symbol comparison --"
    diff <(nm -D --defined-only "$root/c_src/build/libdriver.so" | awk '{print $3}' | sort -u) \
         <(nm -D --defined-only "$DRIVER_RUST_SO" | awk '{print $3}' | sort -u) \
      | grep '^<' && { echo "FAIL: C exports symbols the Rust .so does not"; exit 1; }
    echo "ok: every C export is present in the Rust .so"

    timeout 600 cargo test "${flag[@]}" --no-default-features --features "$combo"
    unset DRIVER_RUST_SO
  done
done

echo
echo "ALL CONFIGURATIONS PASSED"

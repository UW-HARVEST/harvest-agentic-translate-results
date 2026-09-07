#!/usr/bin/env bash
# Builds the C shared library and the Rust cdylib, then runs the differential
# test suite across every feature combination.
#
# `cargo test` does NOT rebuild a `cdylib` artifact, so the explicit
# `cargo build` below is mandatory: without it the tests would load a stale
# libdriver.so. tests/common/mod.rs also asserts freshness as a backstop.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
cargo_flags=(--offline)

echo "== building C shared library =="
mkdir -p "$root/c_src/build"
cmake -S "$root/c_src" -B "$root/c_src/build" -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
cmake --build "$root/c_src/build" >/dev/null
ls -l "$root/c_src/build/libdriver.so"

# The crate declares no [features] table, so the only distinct configurations
# are the default build and --no-default-features (which are equivalent here).
# Enumerate them anyway so new features are picked up automatically.
mapfile -t features < <(
  python3 - "$here/Cargo.toml" <<'PY'
import re, sys
text = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', text, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        k = line.split('=')[0].strip()
        if k and k != 'default':
            names.append(k)
print('')                     # default features
print('__NONE__')             # --no-default-features
for n in names:               # each feature on its own
    print(n)
if len(names) > 1:            # all features together
    print(','.join(names))
PY
)

status=0
for combo in "${features[@]}"; do
  args=("${cargo_flags[@]}" --release)
  if [[ "$combo" == "__NONE__" ]]; then
    label="--no-default-features"
    args+=(--no-default-features)
  elif [[ -z "$combo" ]]; then
    label="(default features)"
  else
    label="--no-default-features --features $combo"
    args+=(--no-default-features --features "$combo")
  fi

  echo
  echo "=============================================================="
  echo "== feature combination: $label"
  echo "=============================================================="

  ( cd "$here" && cargo build "${args[@]}" ) || { status=1; continue; }
  ls -l "$here/target/release/libdriver.so"
  echo "-- exported symbols (Rust) --"
  nm -D --defined-only "$here/target/release/libdriver.so"
  echo "-- symbol diff vs C (empty == parity) --"
  diff <(nm -D --defined-only "$root/c_src/build/libdriver.so" | awk '{print $NF}' | sort) \
       <(nm -D --defined-only "$here/target/release/libdriver.so" | awk '{print $NF}' | sort) \
    && echo "(symbol sets identical)"

  ( cd "$here" && cargo test "${args[@]}" ) || status=1
done

echo
if [[ $status -eq 0 ]]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES DETECTED"
fi
exit $status

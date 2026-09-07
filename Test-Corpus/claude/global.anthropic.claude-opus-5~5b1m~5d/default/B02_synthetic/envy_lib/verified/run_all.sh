#!/usr/bin/env bash
# Full verification run: builds the C .so and the Rust .so, then runs the
# differential suite under every cargo feature combination declared in
# Cargo.toml (plus --no-default-features and --all-features), and finally
# diffs the exported symbol sets.
#
# Usage:  cd translation && bash run_all.sh
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
cd "$root"

echo "=== 1. build the C shared library ==="
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . )
c_so="$(ls c_src/build/lib*.so | head -1)"
echo "C  .so: $c_so"

cd "$here"

echo
echo "=== 2. enumerate feature combinations ==="
# Extract the feature names from the [features] table, if any.
features="$(python3 - <<'PY'
import re
s = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        k = line.split('=')[0].strip().strip('"')
        if k and k != 'default':
            names.append(k)
print(' '.join(names))
PY
)"
echo "declared features: '${features}'"

combos=("default" "--no-default-features")
if [ -n "${features// /}" ]; then
  combos+=("--all-features")
  # every non-empty subset of the declared features, on top of no-default
  read -r -a farr <<< "$features"
  n=${#farr[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    sel=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then sel="$sel,${farr[$i]}"; fi
    done
    combos+=("--no-default-features --features ${sel#,}")
  done
fi

echo
echo "=== 3. cargo check + build + test per combination ==="
fail=0
for combo in "${combos[@]}"; do
  if [ "$combo" = "default" ]; then flags=(); else read -r -a flags <<< "$combo"; fi
  echo
  echo "--- combination: $combo ---"
  cargo check --release "${flags[@]}"
  # The cdylib must be rebuilt for THIS combination before the tests dlopen it.
  cargo build --release "${flags[@]}"
  if timeout 600 cargo test --release "${flags[@]}"; then
    echo "PASS: $combo"
  else
    echo "FAIL: $combo"
    fail=1
  fi
done

echo
echo "=== 3b. cross-check against a -O2 build of the SAME C source ==="
# The C source is never modified; this only builds it a second time with
# optimisation enabled (into translation/target, never into c_src/) to confirm
# the Rust also matches gcc's codegen for the formally-UB arithmetic.
cmake -S "$root/c_src" -B target/c_O2 \
      -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DCMAKE_BUILD_TYPE=Release >/dev/null
cmake --build target/c_O2 >/dev/null
o2_so="$PWD/target/c_O2/$(cd target/c_O2 && ls lib*.so | head -1)"
cargo build --release
if ENVY_C_SO="$o2_so" timeout 600 cargo test --release; then
  echo "PASS: Rust release .so vs C -O2 .so"
else
  echo "FAIL: Rust release .so vs C -O2 .so"; fail=1
fi

echo
echo "=== 3c. cross-check the debug-profile Rust cdylib ==="
# The release profile sets panic = "abort"; the debug profile keeps
# panic = "unwind" AND enables debug_assertions, which is a genuinely different
# build of the same translation.
cargo build
if ENVY_RUST_SO="$PWD/target/debug/libenvy_lib.so" timeout 600 cargo test --release; then
  echo "PASS: C .so vs Rust debug .so"
else
  echo "FAIL: C .so vs Rust debug .so"; fail=1
fi

echo
echo "=== 4. symbol parity (nm -D) ==="
diff <(nm -D --defined-only "$root/$c_so"        | grep -v ' [aVWw] ' | awk '{print $NF}' | sort) \
     <(nm -D --defined-only target/release/libenvy_lib.so | grep -v ' [aVWw] ' | awk '{print $NF}' | sort) \
  && echo "symbol diff: EMPTY (C surface fully exported by Rust)" \
  || { echo "symbol diff NOT EMPTY"; fail=1; }

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL VERIFICATION GATES PASSED"
else
  echo "VERIFICATION FAILED"
fi
exit "$fail"

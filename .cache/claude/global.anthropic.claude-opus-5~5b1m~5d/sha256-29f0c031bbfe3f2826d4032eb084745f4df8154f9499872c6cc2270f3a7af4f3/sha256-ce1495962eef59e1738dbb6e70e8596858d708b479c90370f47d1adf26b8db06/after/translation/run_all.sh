#!/usr/bin/env bash
# Build the C .so and the Rust cdylib, then run the differential suite.
#
# The explicit `cargo build` is REQUIRED: `cargo test` does not rebuild a
# `crate-type = ["cdylib"]` library, because no test target links it. Without
# it the tests would dlopen a stale artifact (the harness now fails loudly
# instead — see tests/common/mod.rs::assert_not_stale).
set -euo pipefail
cd "$(dirname "$0")"

# --- C shared library -------------------------------------------------------
if [ ! -d ../c_src/build ] || [ -z "$(ls ../c_src/build/lib*.so 2>/dev/null)" ]; then
  echo "== building the C shared library =="
  mkdir -p ../c_src/build
  (cd ../c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null)
fi

status=0
for profile in "" "--release"; do
  label=${profile:-debug}
  echo
  echo "=================================================================="
  echo "== profile: ${label#--}  (features: default — the crate defines none)"
  echo "=================================================================="
  timeout 600 cargo build $profile 2>&1 | grep -Ev '^\s*$' | tail -2
  timeout 600 cargo test $profile 2>&1 | grep -E "^(test |running|error|test result|warning: unused)" || status=1
done

echo
echo "== symbol parity =="
./symbol_diff.sh || status=1
exit $status

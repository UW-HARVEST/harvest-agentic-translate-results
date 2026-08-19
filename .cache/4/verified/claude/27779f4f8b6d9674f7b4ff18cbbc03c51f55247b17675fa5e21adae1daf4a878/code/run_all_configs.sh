#!/bin/sh
# Runs the whole differential test-suite for every build configuration.
#
# Build-time configuration axes (see CONFIGS.md):
#   * cargo features — `Cargo.toml` declares only `default = []`, so the
#     complete set of feature combinations is {} and {default}, i.e. the two
#     invocations below;
#   * cargo profile  — dev and release (release also turns on `panic = "abort"`
#     and optimisation, so the executable under test really is a different
#     binary);
#   * the C side has no configuration at all (no `#ifdef`, no cmake option).
set -eu

cd "$(dirname "$0")"

# The C reference artifacts: the cmake-built executable and the same
# translation unit as a shared object.  c_src/ itself is never modified.
echo "=== building the C reference (cmake executable) ==="
mkdir -p c_src/build
(cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null)
echo "=== building the C reference (shared object) ==="
mkdir -p target/cdiff
gcc -shared -fPIC -o target/cdiff/libc_driver.so c_src/src/main.c

status=0
for features in "--no-default-features" "--features default"; do
  for profile in "" "--release"; do
    echo
    echo "=================================================================="
    echo "=== cargo check ${features} ${profile}"
    echo "=================================================================="
    # shellcheck disable=SC2086
    cargo check --offline --all-targets ${features} ${profile} || status=1
    echo
    echo "=================================================================="
    echo "=== cargo test  ${features} ${profile}"
    echo "=================================================================="
    # shellcheck disable=SC2086
    cargo test --offline ${features} ${profile} || status=1
  done
done

echo
if [ "$status" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATION FAILED"
fi
exit "$status"

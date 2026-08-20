#!/usr/bin/env bash
# Runs the whole differential suite in EVERY configuration:
#   {no cargo features} x {Rust dev, Rust release}
# (each run compares against the C ground truth built at both -O0 and -O2, see
# build.rs / tests/common/mod.rs).
#
# The `--ignored` end-to-end test is NOT run here; use
#   E2E_SEEDS=42 cargo test --release --test pipeline -- --ignored --nocapture
# or scripts/e2e_binaries.sh (both take ~5 minutes per implementation).
set -uo pipefail
cd "$(dirname "$0")/.."

fail=0
for profile in release dev; do
  flag=""
  [ "$profile" = "release" ] && flag="--release"
  echo "############################################################"
  echo "# cargo test $flag --no-default-features  (profile: $profile)"
  echo "############################################################"
  # The tests dlopen target/<profile>/libdriver.so; `cargo test` does not build
  # the cdylib itself (the lib target has `test = false`), so build it first.
  # shellcheck disable=SC2086
  cargo build $flag --no-default-features || { echo "BUILD FAILED ($profile)"; exit 1; }

  # --test-threads=1: the exported `array` object and glibc's rand state are
  # process-global, and the error tests redirect fd 1/2.
  # shellcheck disable=SC2086
  if ! cargo test $flag --no-default-features -- --test-threads=1; then
    echo "SUITE FAILED for profile=$profile"
    fail=1
  fi
done

if [ "$fail" -ne 0 ]; then
  echo "DIFFERENTIAL SUITE FAILED"
  exit 1
fi
echo "DIFFERENTIAL SUITE PASSED IN ALL CONFIGURATIONS"

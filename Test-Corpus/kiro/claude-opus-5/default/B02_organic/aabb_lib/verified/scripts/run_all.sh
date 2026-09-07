#!/usr/bin/env bash
# Full differential verification run.
#
#   * builds the C shared library
#   * builds the Rust cdylib (REQUIRED: `cargo test` does not build a
#     cdylib-only lib target, so the tests would otherwise dlopen a stale .so)
#   * diffs `nm -D` between the two
#   * runs every differential test suite under every feature combination
#
# Usage: scripts/run_all.sh
set -uo pipefail

here=$(cd "$(dirname "$0")" && pwd)
crate=$(cd "$here/.." && pwd)
root=$(cd "$crate/.." && pwd)
fail=0

echo "=============================================================="
echo "1. Build the C shared library"
echo "=============================================================="
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . ) || { echo "C BUILD FAILED"; exit 1; }
c_so=$(ls "$root"/c_src/build/lib*.so | head -1)
echo "C  .so: $c_so"

echo
echo "=============================================================="
echo "2. Enumerate feature combinations"
echo "=============================================================="
# Mechanically extract the [features] table; a crate with no [features] has
# exactly one configuration (the default).
features=$(awk '
  /^\[features\]/ { in_f=1; next }
  /^\[/           { in_f=0 }
  in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/,""); print }
' "$crate/Cargo.toml")

combos=()
if [ -z "$features" ]; then
  echo "no [features] table -> configurations: default, --no-default-features"
  combos+=("")                        # default
  combos+=("--no-default-features")   # identical here, checked anyway
else
  echo "features found:"; echo "$features"
  combos+=("")
  combos+=("--no-default-features")
  while read -r f; do
    [ -n "$f" ] && combos+=("--no-default-features --features $f")
  done <<< "$features"
  all=$(echo "$features" | paste -sd, -)
  combos+=("--all-features")
  combos+=("--no-default-features --features $all")
fi

for combo in "${combos[@]}"; do
 for profile in dev release; do
  label="${combo:-"(default features)"} / $profile"
  if [ "$profile" = release ]; then pflag="--release"; outdir=release; else pflag=""; outdir=debug; fi
  echo
  echo "=============================================================="
  echo "3. Configuration: $label"
  echo "=============================================================="

  # NOTE: `cargo test` never builds a cdylib-only lib target, so the .so must
  # be built explicitly or the tests would dlopen a stale artefact.
  echo "--- cargo build $combo $pflag ---"
  ( cd "$crate" && timeout 600 cargo build $combo $pflag ) \
    || { echo "RUST BUILD FAILED ($label)"; fail=1; continue; }

  r_so="$crate/target/$outdir/libaabb_lib.so"
  echo "Rust .so: $r_so"

  echo "--- nm -D symbol parity ---"
  nm -D --defined-only "$c_so" | awk '{print $3}' | sort > /tmp/c_syms.txt
  nm -D --defined-only "$r_so" | awk '{print $3}' | sort > /tmp/r_syms.txt
  missing=$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)
  extra=$(comm -13 /tmp/c_syms.txt /tmp/r_syms.txt)
  echo "C defines $(wc -l < /tmp/c_syms.txt) symbols; Rust defines $(wc -l < /tmp/r_syms.txt)"
  if [ -n "$missing" ]; then echo "MISSING IN RUST:"; echo "$missing"; fail=1; else echo "missing in Rust: none"; fi
  if [ -n "$extra" ]; then echo "extra in Rust (informational):"; echo "$extra"; fi

  echo "--- undefined non-libc symbols in the Rust .so ---"
  und=$(nm -D --undefined-only "$r_so" | awk '{print $NF}' \
        | grep -v '@GLIBC\|@GCC\|^_ITM_\|^__gmon_start__\|^_Unwind_\|^__cxa_\|^__tls_get_addr' || true)
  if [ -n "$und" ]; then echo "UNRESOLVED:"; echo "$und"; fail=1; else echo "none"; fi

  echo "--- cargo test $combo $pflag ---"
  ( cd "$crate" && timeout 600 cargo test $combo $pflag -- --test-threads=4 ) \
    || { echo "TESTS FAILED ($label)"; fail=1; }
 done
done

echo
echo "=============================================================="
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
echo "=============================================================="
exit "$fail"

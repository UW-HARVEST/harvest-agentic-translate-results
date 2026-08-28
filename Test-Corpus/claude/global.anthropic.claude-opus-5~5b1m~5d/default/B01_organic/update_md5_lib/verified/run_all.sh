#!/usr/bin/env bash
# Full verification run: build the C reference .so, then build+test the Rust
# cdylib under EVERY feature combination and BOTH profiles, and diff the
# exported symbol tables.
#
#   ./run_all.sh
#
set -uo pipefail
cd "$(dirname "$0")"
ROOT=$(pwd)/..
rc=0

echo "=============== 1. build the C reference shared library ==============="
(
  cd "$ROOT/c_src" && mkdir -p build && cd build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null
) || { echo "C build FAILED"; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/lib*.so | head -1)
echo "C .so: $C_SO"

# Feature combinations: this crate declares no [features], so the complete set
# of configurations is {default} == {--no-default-features}. Both are exercised
# (and --all-features, which is a no-op here) so the loop stays correct if a
# feature is ever added.
FEATURE_SETS=("" "--no-default-features" "--all-features")

for profile in "" "--release"; do
  for feats in "${FEATURE_SETS[@]}"; do
    label="profile='${profile:-dev}' features='${feats:-default}'"
    echo
    echo "=============== 2. cargo check/build/test :: $label ==============="
    if ! timeout 600 cargo check $profile $feats 2>&1 | tail -3; then
      echo "CHECK FAILED :: $label"; rc=1; continue
    fi
    # `cargo test` does not build cdylib artifacts, so build the .so first.
    if ! timeout 600 cargo build $profile $feats 2>&1 | tail -3; then
      echo "BUILD FAILED :: $label"; rc=1; continue
    fi
    log=$(mktemp)
    if timeout 600 cargo test $profile $feats >"$log" 2>&1; then
      grep -E "^(     Running|test result:)" "$log"
    else
      echo "TESTS FAILED :: $label"
      grep -E "^(test result:|failures:|---- )" "$log" | head -40
      tail -20 "$log"
      rc=1
    fi
    rm -f "$log"
  done
done

echo
echo "=============== 3. symbol parity (nm -D) ==============="
for rust_so in target/debug/libupdate_md5_lib.so target/release/libupdate_md5_lib.so; do
  [ -f "$rust_so" ] || continue
  c_syms=$(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort)
  r_syms=$(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  echo "--- $rust_so"
  echo "C exports:    $(echo "$c_syms" | tr '\n' ' ')"
  if [ -z "$missing" ]; then
    echo "missing from Rust: <none>"
  else
    echo "MISSING FROM RUST: $missing"; rc=1
  fi
  undef=$(nm -D --undefined-only "$rust_so" | awk '{print $2}' |
    grep -vE '^(_Unwind_|__|_ITM_|abort@|bcmp@|calloc@|close@|dl_iterate_phdr@|free@|fstat|getcwd@|getenv@|gettid@|lseek|malloc@|memcpy@|memmove@|memset@|mmap|munmap@|open|posix_memalign@|pthread_|read@|readlink@|realloc@|realpath@|stat|statx@|strlen@|syscall@|write@|writev@)' || true)
  if [ -n "$undef" ]; then
    echo "NON-LIBC UNDEFINED SYMBOLS: $undef"; rc=1
  else
    echo "non-libc undefined symbols: <none>"
  fi
done

echo
if [ "$rc" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$rc"

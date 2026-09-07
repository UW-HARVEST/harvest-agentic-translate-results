#!/usr/bin/env bash
# Full verification driver: builds the C .so and the Rust cdylib, diffs the
# exported symbol sets, and runs the differential suite under every feature
# combination and both profiles.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
fail=0
note() { printf '\n=== %s ===\n' "$*"; }

note "building C shared library"
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
c_so="$(ls "$root"/c_src/build/*.so | head -1)"
echo "C  .so: $c_so"

note "building Rust cdylib (debug + release)"
( cd "$here" && cargo build --offline --lib >/dev/null 2>&1 ) || { echo "rust debug build FAILED"; exit 1; }
( cd "$here" && cargo build --offline --lib --release >/dev/null 2>&1 ) || { echo "rust release build FAILED"; exit 1; }

note "symbol parity (nm -D)"
syms() { nm -D --defined-only "$1" | awk '$2=="T"||$2=="t" {print $3}' | sort -u; }
for profile in debug release; do
  r_so="$here/target/$profile/libcleanup_lib.so"
  echo "--- Rust $profile: $r_so"
  missing="$(comm -23 <(syms "$c_so") <(syms "$r_so"))"
  if [ -n "$missing" ]; then
    echo "MISSING from Rust $profile .so:"; echo "$missing"; fail=1
  else
    echo "OK: 0 symbols missing ($(syms "$c_so" | wc -l) C exports all present)"
  fi
done

note "undefined non-libc symbols in the Rust .so"
# `nm -D -u` renders versioned imports as `name@GLIBC_2.2.5`; strip the version
# suffix before matching, otherwise every libc import looks unrecognised.
undef="$(nm -D -u "$here/target/release/libcleanup_lib.so" \
  | awk '{print $NF}' | sed 's/@@*.*$//' | sort -u \
  | grep -vE '^(printf|snprintf|puts|putchar|fwrite|malloc|free|calloc|realloc|posix_memalign|strncmp|strlen|strcmp|memcpy|memmove|memset|memcmp|bcmp|abort|exit|write|writev|open|open64|close|read|readlink|lseek64|stat64|statx|realpath|getcwd|getenv|syscall|gettid|sysconf|mmap|mmap64|munmap|mremap|poll|sigaltstack|sigaction|sigaddset|sigemptyset|fstat64|fstat|fcntl|fcntl64|_Unwind_.*|__.*|pthread_.*|dl.*|rust_eh_personality\
|_ITM_registerTMCloneTable|_ITM_deregisterTMCloneTable)$' || true)"
if [ -n "$undef" ]; then echo "UNRESOLVED non-libc symbols:"; echo "$undef"; fail=1; else echo "OK: none"; fi

note "feature combinations declared in Cargo.toml"
feat="$(sed -n '/^\[features\]/,/^\[/p' "$here/Cargo.toml" | grep -E '^[a-zA-Z0-9_-]+ *=' | cut -d= -f1 | tr -d ' ')"
if [ -z "$feat" ]; then
  echo "no [features] table -> combinations: default, --no-default-features, --all-features"
  combos=("" "--no-default-features" "--all-features")
else
  echo "features: $feat"
  combos=("" "--no-default-features" "--all-features")
  for f in $feat; do combos+=("--no-default-features --features $f"); done
fi

for profile_flag in "" "--release"; do
  for combo in "${combos[@]}"; do
    note "cargo test --offline $profile_flag $combo"
    out="$(cd "$here" && timeout 600 cargo test --offline $profile_flag $combo 2>&1)"
    echo "$out" | grep -E 'test result:' || true
    if echo "$out" | grep -qE 'FAILED|error(\[|:)'; then
      echo "^^^ FAILURE in [$profile_flag $combo]"; fail=1
    fi
  done
done

note "result"
if [ "$fail" -eq 0 ]; then echo "ALL VERIFICATION CHECKS PASSED"; else echo "VERIFICATION FAILED"; fi
exit "$fail"

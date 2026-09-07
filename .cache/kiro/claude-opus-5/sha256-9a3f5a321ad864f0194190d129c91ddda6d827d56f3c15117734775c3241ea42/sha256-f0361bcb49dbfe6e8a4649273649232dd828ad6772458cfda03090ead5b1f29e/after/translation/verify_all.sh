#!/usr/bin/env bash
# Phase D driver: rebuild both sides, diff exported symbols, then run the full
# Phase B + Phase C suite across every Cargo feature combination AND against
# both the release and debug Rust `.so` (the latter has `overflow-checks` on,
# which is what catches divergence in the wrapping allocation-size arithmetic).
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
C_SO="$ROOT/c_src/build/libdriver.so"
fail=0

echo "=== rebuild C ==="
( cd "$ROOT/c_src/build" && cmake --build . ) >/dev/null || { echo "C build FAILED"; exit 1; }

echo
echo "=== feature combinations declared in Cargo.toml ==="
# Enumerate the powerset of declared features. This crate declares none, so the
# space is exactly {default, no-default-features}; the loop is generic anyway.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+ *=/{print $1}' Cargo.toml)
COMBOS=()
COMBOS+=("--all-features")
COMBOS+=("")                       # default features
COMBOS+=("--no-default-features")
if [ -n "$FEATURES" ]; then
  feats=($FEATURES)
  n=${#feats[@]}
  for ((m=1; m<(1<<n); m++)); do
    sel=""
    for ((i=0; i<n; i++)); do
      if (( m & (1<<i) )); then sel="${sel:+$sel,}${feats[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features $sel")
  done
else
  echo "(none declared)"
fi
printf '  combo: %s\n' "${COMBOS[@]/#/[}" | sed 's/\[$/[<default>/'

for combo in "${COMBOS[@]}"; do
  for profile in release debug; do
    label="features='${combo:-<default>}' profile=$profile"
    echo
    echo "############ $label ############"

    if [ "$profile" = release ]; then
      timeout 600 cargo build --release $combo >/dev/null 2>&1 || { echo "BUILD FAILED: $label"; fail=1; continue; }
      RS_SO="$PWD/target/release/libdriver.so"
      PROFILE_FLAG="--release"
    else
      timeout 600 cargo build $combo >/dev/null 2>&1 || { echo "BUILD FAILED: $label"; fail=1; continue; }
      RS_SO="$PWD/target/debug/libdriver.so"
      PROFILE_FLAG=""
    fi

    echo "--- symbol diff (C .so vs Rust .so) ---"
    cdefs=$(nm -D --defined-only "$C_SO"  | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' | sort -u)
    rdefs=$(nm -D --defined-only "$RS_SO" | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' | sort -u)
    missing=$(comm -23 <(echo "$cdefs") <(echo "$rdefs"))
    if [ -n "$missing" ]; then
      echo "MISSING FROM RUST .so:"; echo "$missing"; fail=1
    else
      echo "OK: 0 symbols missing ($(echo "$cdefs" | wc -l) C symbols all present)"
    fi

    echo "--- undefined non-libc symbols in Rust .so ---"
    und=$(nm -D --undefined-only "$RS_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u \
          | grep -v -E '^(calloc|malloc|free|realloc|posix_memalign|memcpy|memmove|memset|memcmp|bcmp|strlen|strrchr|strerror|fputs|fprintf|fwrite|exit|_exit|abort|raise|__errno_location|stderr|__stack_chk_fail|__cxa_[a-z_]*|_ITM_[a-zA-Z]*|__gmon_start__|__tls_get_addr|_Unwind_[a-zA-Z]*|rust_eh_personality|__libc_start_main|dl_iterate_phdr|pthread_[a-z_]*|__[a-z_]*_chk|getenv|getcwd|gettid|realpath|readlink|write|writev|read|open|open64|close|lseek64|fstat64|stat64|statx|poll|syscall|sigaltstack|sigaction|sigemptyset|mmap|mmap64|munmap|mprotect|__register_atfork|gnu_get_libc_version)$' || true)
    if [ -n "$und" ]; then echo "NOTE (review): $und"; else echo "OK: none (all imports are libc)"; fi

    echo "--- Phase B + Phase C differential tests ---"
    RUST_SO="$RS_SO" C_SO="$C_SO" timeout 600 cargo test $PROFILE_FLAG $combo 2>&1 \
      | grep -E '^(running|test result|error|failures:|test .* FAILED)' \
      | sed 's/^/    /'
    rc=${PIPESTATUS[0]}
    if [ "$rc" -ne 0 ]; then echo "TESTS FAILED: $label"; fail=1; fi
  done
done

echo
echo "--- binary executable gate ---"
if grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt" 2>/dev/null; then
  echo "C declares an executable -- stdout comparison REQUIRED"; fail=1
else
  echo "N/A: c_src/CMakeLists.txt declares no add_executable, Cargo.toml declares no [[bin]]"
fi

echo
if [ "$fail" -eq 0 ]; then echo "ALL PHASE D CHECKS PASSED"; else echo "PHASE D FAILURES PRESENT"; fi
exit $fail

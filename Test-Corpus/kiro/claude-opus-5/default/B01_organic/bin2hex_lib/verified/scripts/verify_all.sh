#!/usr/bin/env bash
# Phase D automation: symbol parity + every feature combination x profile.
#
# Usage: ./scripts/verify_all.sh
# Run from the `translation/` directory (or anywhere; paths are resolved).
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
cd "$HERE"

fail=0
note() { printf '%s\n' "$*"; }
bad() { printf 'FAIL: %s\n' "$*"; fail=1; }

# --------------------------------------------------------------------------
# 1. Build the C shared library
# --------------------------------------------------------------------------
note "== building C .so =="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || bad "C build"

C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)"
[ -n "$C_SO" ] || bad "no C .so produced"
note "C  .so: $C_SO"

# --------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml
# --------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {sub(/[ \t]*=.*/,""); gsub(/[ \t]/,""); if ($0 != "" && $0 != "default") print}' Cargo.toml
)
note "== declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)} =="

# Build the list of cargo feature-flag sets to test: the default build, plus
# --no-default-features, plus --all-features, plus every subset of the declared
# features (power set) when any exist.
COMBOS=("")
COMBOS+=("--no-default-features")
COMBOS+=("--all-features")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[$i]}")
    done
    if [ ${#sel[@]} -gt 0 ]; then
      COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
fi

# --------------------------------------------------------------------------
# 3. For each combination x profile: build, diff symbols, run all tests
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  for prof in dev release; do
    pflag=""; pdir="debug"
    [ "$prof" = release ] && { pflag="--release"; pdir="release"; }
    label="profile=$prof features=[${combo:-default}]"
    note "== $label =="

    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $pflag $combo >/dev/null 2>&1; then
      bad "$label: cargo build"
      continue
    fi

    R_SO="target/$pdir/libbin2hex_lib.so"
    [ -f "$R_SO" ] || { bad "$label: $R_SO missing"; continue; }

    # Symbol parity: every dynamic symbol defined by the C .so must also be
    # defined by the Rust .so, with the exact same name.
    missing="$(comm -23 \
      <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u) \
      <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u))"
    if [ -n "$missing" ]; then
      bad "$label: symbols missing from Rust .so: $(echo "$missing" | tr '\n' ' ')"
    else
      note "  symbol diff: empty"
    fi

    # Undefined non-libc symbols in the Rust .so.
    undef="$(nm -D --undefined-only "$R_SO" | awk '{print $2}' | sed 's/@.*//' \
      | grep -vE '^(_|__)' | grep -vE '^(abort|memcpy|memset|memmove|malloc|free|realloc|calloc|write|read|close|open|mmap|munmap|mprotect|pthread_[a-z_]+|dl[a-z_]+|sys[a-z_]+|getenv|strlen|memcmp|bcmp|signal|sigaction|raise|gettid|syscall|poll|sched_[a-z_]+|clock_gettime|nanosleep|futex.*|exit|_exit|environ|stat|fstat|lseek|dup|dup2|pipe2|readlink|getcwd|posix_[a-z_]+|__errno_location|fstat64|lseek64|mmap64|open64|realpath|stat64|statx|writev|fstat|fstatat|fstatat64|openat|openat64|pread64|pwrite64|ftruncate64|getrandom|sigaltstack|abort_handler)$' || true)"
    if [ -n "$undef" ]; then
      note "  undefined (review): $(echo "$undef" | tr '\n' ' ')"
    else
      note "  undefined non-libc symbols: none"
    fi

    # shellcheck disable=SC2086
    out="$(timeout 600 cargo test $pflag $combo 2>&1)"
    if echo "$out" | grep -q "FAILED\|error\[" ; then
      bad "$label: tests"
      echo "$out" | grep -E "FAILED|differs|panicked" | head -20
    else
      echo "$out" | grep -E "^test result:" | sed 's/^/  /'
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL PHASE D CHECKS PASSED"
else
  echo "PHASE D CHECKS FAILED"
fi
exit "$fail"

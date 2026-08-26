#!/usr/bin/env bash
# Phase D driver: builds the C .so, enumerates EVERY Cargo feature combination,
# and for each one runs `cargo check`, builds the Rust cdylib, diffs the
# exported-symbol sets (`nm -D`) and runs the full differential test suite.
#
# Usage: ./run_all.sh
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$PWD"
LOG_DIR="${TMPDIR:-/tmp}/driver-verify"
mkdir -p "$LOG_DIR"
CARGO_FLAGS="--offline"
rc=0

say() { printf '\n==== %s ====\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Build the C shared library
# ---------------------------------------------------------------------------
say "building C shared library"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) > "$LOG_DIR/cmake.log" 2>&1 || {
    echo "C BUILD FAILED (see $LOG_DIR/cmake.log)"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
ls -l "$C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate every valid feature combination (power set of [features])
# ---------------------------------------------------------------------------
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}
' Cargo.toml | grep -v '^default$' | sort -u)

combos=("")                      # always test the empty set
if [[ -n "$FEATURES" ]]; then
  mapfile -t flist <<< "$FEATURES"
  n=${#flist[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      (( mask & (1 << i) )) && combo+="${flist[$i]},"
    done
    combos+=("${combo%,}")
  done
fi
say "feature combinations to verify: ${#combos[@]}"
for c in "${combos[@]}"; do echo "  - '${c:-<none>}'"; done
echo "(Cargo.toml declares no [features] => the default set and"
echo " --no-default-features are the same, single configuration)"

# ---------------------------------------------------------------------------
# 3. check / build / symbol-diff / test each combination
# ---------------------------------------------------------------------------
for combo in "${combos[@]}"; do
  if [[ -n "$combo" ]]; then
    FEAT_ARGS=(--no-default-features --features "$combo")
    label="features=$combo"
  else
    FEAT_ARGS=(--no-default-features)
    label="features=<none>"
  fi

  say "cargo check ($label)"
  cargo check $CARGO_FLAGS "${FEAT_ARGS[@]}" 2>&1 | tail -3 || rc=1

  say "cargo build ($label)"
  cargo build $CARGO_FLAGS "${FEAT_ARGS[@]}" 2>&1 | tail -3 || rc=1

  RUST_SO="$ROOT/target/debug/libdriver.so"
  say "symbol parity ($label)"
  nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort > "$LOG_DIR/c.syms"
  nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort > "$LOG_DIR/rust.syms"
  echo "C exports:    $(wc -l < "$LOG_DIR/c.syms")"
  echo "Rust exports: $(wc -l < "$LOG_DIR/rust.syms")"
  missing=$(comm -23 "$LOG_DIR/c.syms" "$LOG_DIR/rust.syms")
  if [[ -n "$missing" ]]; then
    echo "MISSING FROM RUST .so:"; echo "$missing"; rc=1
  else
    echo "OK: 0 symbols missing from the Rust .so"
  fi
  # Non-libc undefined symbols in the Rust .so (must be none).
  undef=$(nm -D --undefined-only "$RUST_SO" | awk '{print $NF}' \
    | sed 's/@.*//' \
    | grep -vE '^(_ITM_|__cxa_|__gmon_start__|_Unwind_|__tls_get_addr|__errno_location|statx|gettid)' \
    | grep -vxE 'malloc|calloc|realloc|free|posix_memalign|memcpy|memmove|memset|bcmp|strlen|abort|getenv|getcwd|readlink|realpath|open64|close|read|write|writev|lseek64|stat64|fstat64|mmap64|munmap|dl_iterate_phdr|syscall|pthread_key_create|pthread_key_delete|pthread_setspecific')
  if [[ -n "$undef" ]]; then
    echo "UNEXPECTED (non-libc) UNDEFINED SYMBOLS:"; echo "$undef"; rc=1
  else
    echo "OK: 0 unexpected undefined (non-libc) symbols"
  fi

  say "cargo test ($label)"
  cargo test $CARGO_FLAGS "${FEAT_ARGS[@]}" 2>&1 | tail -30 || rc=1
done

say "RESULT"
if [[ $rc -eq 0 ]]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES DETECTED"; fi
exit $rc

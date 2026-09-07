#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination + both profiles.
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
C_SO="$(ls "$ROOT"/c_src/build/*.so | head -1)"
FAIL=0

echo "=============================================================="
echo "Phase D.1 — symbol parity (nm -D)"
echo "=============================================================="
echo "C   .so: $C_SO"

for PROFILE in debug release; do
  FLAG=""; [ "$PROFILE" = release ] && FLAG="--release"
  timeout 600 cargo build $FLAG >/dev/null 2>&1 || { echo "build $PROFILE FAILED"; FAIL=1; continue; }
  R_SO="target/$PROFILE/libupdate_md5_lib.so"
  echo "--- profile: $PROFILE ($R_SO) ---"

  nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > /tmp/c.syms
  nm -D --defined-only "$R_SO" | awk '{print $3}' | grep -v '^_ZN' | sort -u > /tmp/r.syms

  MISSING="$(comm -23 /tmp/c.syms /tmp/r.syms)"
  echo "C exports ($(wc -l < /tmp/c.syms)): $(tr '\n' ' ' < /tmp/c.syms)"
  if [ -n "$MISSING" ]; then
    echo "  *** MISSING FROM RUST .so: $MISSING"
    FAIL=1
  else
    echo "  symbol diff (C -> Rust): EMPTY  [OK]"
  fi

  # Undefined symbols in the Rust .so that are not libc / libgcc-unwinder.
  BAD="$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' \
        | sed 's/@.*//' \
        | grep -vE '^(_ITM_(de)?registerTMCloneTable|__gmon_start__|__cxa_finalize|__cxa_thread_atexit_impl)$' \
        | grep -vE '^_Unwind_' \
        | grep -vE '^(malloc|free|calloc|realloc|posix_memalign|memcpy|memmove|memset|bcmp|strlen|abort|getenv|getcwd|open64|read|write|close|lseek64|stat64|fstat64|statx|mmap64|munmap|readlink|realpath|syscall|writev|dl_iterate_phdr|__errno_location|__tls_get_addr|pthread_key_create|pthread_key_delete|pthread_setspecific|gettid|sysconf|__libc_start_main)$')"
  if [ -n "$BAD" ]; then
    echo "  *** NON-LIBC UNDEFINED SYMBOLS: $BAD"
    FAIL=1
  else
    echo "  undefined non-libc/non-unwinder symbols: NONE  [OK]"
  fi
done

echo
echo "=============================================================="
echo "Phase D.2 — feature combinations"
echo "=============================================================="
FEATS="$(cargo metadata --no-deps --format-version 1 2>/dev/null \
        | tr ',' '\n' | grep -o '"features":{[^}]*}' | head -1)"
echo "cargo metadata features: ${FEATS:-<none>}"
if grep -q '^\[features\]' Cargo.toml; then
  echo "  *** Cargo.toml declares [features]; enumerate and loop over them."
  FAIL=1
else
  echo "  Cargo.toml declares no [features] table."
fi
# Enumerate combos mechanically anyway: the only combo is the empty one.
COMBOS=("")
for COMBO in "${COMBOS[@]}"; do
  LABEL="${COMBO:-<default/none>}"
  echo "--- combo: $LABEL ---"
  if [ -z "$COMBO" ]; then
    timeout 600 cargo check --no-default-features >/dev/null 2>&1 \
      && echo "  cargo check --no-default-features: OK" \
      || { echo "  cargo check --no-default-features: FAILED"; FAIL=1; }
  else
    timeout 600 cargo check --no-default-features --features "$COMBO" >/dev/null 2>&1 \
      && echo "  cargo check --features $COMBO: OK" \
      || { echo "  cargo check --features $COMBO: FAILED"; FAIL=1; }
  fi
done

echo
echo "=============================================================="
echo "Phase D.3 — Phases B+C under every profile x feature combo"
echo "=============================================================="
for PROFILE in debug release; do
  FLAG=""; [ "$PROFILE" = release ] && FLAG="--release"
  for COMBO in "${COMBOS[@]}"; do
    LABEL="${COMBO:-none}"
    timeout 600 cargo build $FLAG --no-default-features >/dev/null 2>&1
    export RUST_SO_PATH="$PWD/target/$PROFILE/libupdate_md5_lib.so"
    export C_SO_PATH="$C_SO"
    echo "--- profile=$PROFILE features=$LABEL  (Rust .so: $RUST_SO_PATH) ---"
    OUT="$(timeout 600 cargo test $FLAG --no-default-features 2>&1)"
    echo "$OUT" | grep -E '^test result' | sed 's/^/  /'
    if echo "$OUT" | grep -qE 'FAILED|error\['; then
      echo "$OUT" | grep -E 'FAILED|^---- |error\[' | head -20 | sed 's/^/  /'
      FAIL=1
    fi
    unset RUST_SO_PATH C_SO_PATH
  done
done

echo
echo "=============================================================="
if [ "$FAIL" -eq 0 ]; then
  echo "PHASE D: ALL CHECKS PASSED"
else
  echo "PHASE D: FAILURES PRESENT"
fi
echo "=============================================================="
exit "$FAIL"

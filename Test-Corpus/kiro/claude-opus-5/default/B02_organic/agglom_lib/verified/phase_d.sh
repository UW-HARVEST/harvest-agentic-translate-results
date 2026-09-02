#!/usr/bin/env bash
# Phase D — symbol parity + the full configuration matrix.
# Run from the crate root (translation/).
set -uo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
WS="$(dirname "$ROOT")"
CSO="$(ls "$WS"/c_src/build/lib*.so | head -1)"
FAIL=0

echo "=============================================================="
echo "Phase D.1 — symbol parity (nm -D)"
echo "=============================================================="
for PROFILE in release debug; do
  if [ "$PROFILE" = release ]; then
    timeout 600 cargo build --release >/dev/null 2>&1
  else
    timeout 600 cargo build >/dev/null 2>&1
  fi
  RSO="$ROOT/target/$PROFILE/libagglom_lib.so"
  nm -D --defined-only "$CSO" | awk '{print $3}' | sort -u > /tmp/pd_c.txt
  nm -D --defined-only "$RSO" | awk '{print $3}' | sort -u > /tmp/pd_r.txt
  MISSING="$(comm -23 /tmp/pd_c.txt /tmp/pd_r.txt)"
  NC=$(wc -l < /tmp/pd_c.txt)
  echo "[$PROFILE] C exports: $NC"
  if [ -n "$MISSING" ]; then
    echo "[$PROFILE] MISSING FROM RUST:"; echo "$MISSING"; FAIL=1
  else
    echo "[$PROFILE] missing from Rust: 0  -> PARITY OK"
  fi
  # undefined symbols that are not libc / libgcc-unwind
  UNDEF="$(nm -D -u "$RSO" | awk '{print $2}' \
    | grep -vE '^(_ITM_|_Unwind_|__cxa_|__gmon_|__tls_get_addr|__errno_location|statx|gettid)' \
    | grep -vE '^(abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|strlen|syscall|write|writev)@' \
    | sed '/^$/d')"
  if [ -n "$UNDEF" ]; then
    echo "[$PROFILE] non-libc undefined symbols:"; echo "$UNDEF"; FAIL=1
  else
    echo "[$PROFILE] non-libc undefined symbols: 0  -> OK"
  fi
done

echo
echo "=============================================================="
echo "Phase D.2 — enumerate feature combinations from Cargo.toml"
echo "=============================================================="
FEATURES="$(python3 - <<'PY'
import tomllib
m = tomllib.load(open('Cargo.toml','rb'))
f = m.get('features') or {}
names = [k for k in f if k != 'default']
print(' '.join(names))
PY
)"
if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares NO [features] table and no optional dependencies."
  echo "The complete matrix is therefore: default == --no-default-features == --all-features"
  COMBOS=("" "--no-default-features" "--all-features")
else
  echo "features: $FEATURES"
  COMBOS=("" "--no-default-features" "--all-features")
  for f in $FEATURES; do
    COMBOS+=("--no-default-features --features $f")
  done
fi

echo
echo "=============================================================="
echo "Phase D.3 — run Phases B+C under every combination x profile"
echo "=============================================================="
for PROFILE_FLAG in "--release" ""; do
  PNAME=$([ -n "$PROFILE_FLAG" ] && echo release || echo debug)
  for COMBO in "${COMBOS[@]}"; do
    LABEL="profile=$PNAME combo=${COMBO:-<default>}"
    echo "-------- $LABEL --------"
    # shellcheck disable=SC2086
    if timeout 600 cargo test $PROFILE_FLAG $COMBO 2>&1 | tee /tmp/pd_test.log \
        | grep -E '^test result:'; then
      if grep -qE 'FAILED|panicked|error\[|error:' /tmp/pd_test.log; then
        echo "!! FAILURES under $LABEL"; grep -E 'FAILED|panicked' /tmp/pd_test.log | head -20
        FAIL=1
      fi
    else
      echo "!! cargo test produced no result line under $LABEL"
      tail -20 /tmp/pd_test.log
      FAIL=1
    fi
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
exit $FAIL

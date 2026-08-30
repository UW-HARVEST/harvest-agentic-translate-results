#!/usr/bin/env bash
# Phase D: full verification matrix.
#
#   1. rebuild the C .so and the Rust .so (debug + release)
#   2. nm -D symbol parity gate (SYMBOLS.md)
#   3. mechanically enumerate every [features] combination from Cargo.toml and
#      run cargo check + the differential suite for each
#   4. harness self-validation via mutation_check.sh
#
# Usage: bash verify_all.sh [--skip-mutation]
set -uo pipefail
cd "$(dirname "$0")" || exit 1
ROOT=$(cd .. && pwd)
T="${TMPDIR:-/tmp}"; mkdir -p "$T"
rc=0
step() { echo; echo "########## $* ##########"; }

# --- 1. builds ------------------------------------------------------------
step "1. building C shared library"
( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C BUILD FAILED"; exit 1; }

step "2. building Rust cdylib (debug + release) and the runner"
cargo build --offline -q            || { echo "rust debug build FAILED";   rc=1; }
cargo build --offline -q --release   || { echo "rust release build FAILED"; rc=1; }
cargo build --offline -q --example runner || { echo "runner build FAILED";  rc=1; }
# cargo can hard-link a cached artifact with an older mtime than src/lib.rs;
# the suite's staleness guard would then fire spuriously. These builds just
# confirmed the artifacts are up to date, so stamp them.
touch target/debug/libdriver.so target/release/libdriver.so

# --- 2. symbol parity -----------------------------------------------------
step "3. nm -D symbol parity (SYMBOLS.md gate)"
CLIB="$ROOT/c_src/build/libdriver.so"
nm -D --defined-only "$CLIB" | awk 'NF>=3{print $3}' | sort -u > "$T/c.syms"
echo "C exports: $(tr '\n' ' ' < "$T/c.syms")"
for prof in debug release; do
  RLIB="target/$prof/libdriver.so"
  [ -f "$RLIB" ] || { echo "  $prof: MISSING $RLIB"; rc=1; continue; }
  nm -D --defined-only "$RLIB" | awk 'NF>=3{print $3}' | sort -u > "$T/r.syms"
  missing=$(comm -23 "$T/c.syms" "$T/r.syms")
  if [ -n "$missing" ]; then
    echo "  $prof: MISSING SYMBOLS -> $(echo "$missing" | tr '\n' ' ')"; rc=1
  else
    echo "  $prof: OK - 0 missing symbols"
  fi
  # No non-libc undefined symbols.
  bad=$(nm -D -u "$RLIB" | awk 'NF>=2{print $2}' \
        | grep -vE '^(_ITM_|__cxa_|__gmon_|_Unwind_|__tls_get_addr|__errno_location|statx|gettid)' \
        | sed 's/@.*//' | sort -u \
        | grep -vE '^(printf|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|strlen|syscall|write|writev)$')
  if [ -n "$bad" ]; then
    echo "  $prof: unexpected non-libc undefined symbols -> $(echo "$bad" | tr '\n' ' ')"; rc=1
  else
    echo "  $prof: OK - 0 undefined non-libc symbols"
  fi
done

# --- 3. feature matrix ----------------------------------------------------
step "4. feature-combination matrix"
mapfile -t FEATS < <(python3 - <<'PY'
import re
s = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*(.*?)(?=^\[|\Z)', s, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n and n != 'default':
                names.append(n)
print('\n'.join(names))
PY
)
if [ "${#FEATS[@]}" -eq 0 ] || [ -z "${FEATS[0]:-}" ]; then
  echo "Cargo.toml declares no [features]; the complete combination set is:"
  COMBOS=("<default>" "--no-default-features")
else
  echo "features found: ${FEATS[*]}"
  COMBOS=("<default>" "--no-default-features")
  n=${#FEATS[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    sel=()
    for ((i=0; i<n; i++)); do (( mask & (1<<i) )) && sel+=("${FEATS[i]}"); done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
fi

for combo in "${COMBOS[@]}"; do
  flags=""; [ "$combo" != "<default>" ] && flags="$combo"
  echo
  echo "--- combination: $combo"
  # shellcheck disable=SC2086
  cargo check --offline -q $flags || { echo "    cargo check FAILED"; rc=1; continue; }
  # shellcheck disable=SC2086
  cargo build --offline -q $flags && cargo build --offline -q --release $flags
  touch target/debug/libdriver.so target/release/libdriver.so
  # shellcheck disable=SC2086
  out=$(timeout 600 cargo test --offline $flags 2>&1)
  if echo "$out" | grep -q "test result: FAILED" || ! echo "$out" | grep -q "test result: ok"; then
    echo "    TESTS FAILED for combination: $combo"
    echo "$out" | grep -E "test result:|^ *(cfg|err|symbols)_[a-z0-9_]+$" | head -20
    rc=1
  else
    echo "    tests: $(echo "$out" | grep -c '^test .* ok$') passed"
  fi
done

# --- 4. harness self-validation ------------------------------------------
if [ "${1:-}" != "--skip-mutation" ]; then
  step "5. harness self-validation (mutation testing)"
  timeout 600 bash mutation_check.sh || { echo "MUTATION CHECK FAILED"; rc=1; }
fi

step "RESULT"
[ "$rc" -eq 0 ] && echo "ALL PHASES PASSED" || echo "FAILURES PRESENT (rc=$rc)"
exit "$rc"

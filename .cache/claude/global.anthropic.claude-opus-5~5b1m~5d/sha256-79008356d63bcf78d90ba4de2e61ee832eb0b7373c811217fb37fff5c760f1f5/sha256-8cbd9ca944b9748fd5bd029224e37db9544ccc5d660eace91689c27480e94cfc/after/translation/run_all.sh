#!/usr/bin/env bash
# Full verification run: build the C .so, check symbol parity, and run the
# differential suite under EVERY feature combination declared in Cargo.toml.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------- C library
step "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
ls -l "$C_SO"

# --------------------------------------------------- enumerate feature combos
# Every subset of the [features] table, plus the default and no-default builds.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

declare -a COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "No [features] table in Cargo.toml -> single build configuration."
  COMBOS+=("DEFAULT" "NODEFAULT")
else
  echo "Features found: ${FEATURES[*]}"
  COMBOS+=("DEFAULT" "NODEFAULT")
  n=${#FEATURES[@]}
  for (( mask=1; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

# ------------------------------------------------------------------ run them
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    DEFAULT)   ARGS=() ;              LABEL="default features" ;;
    NODEFAULT) ARGS=(--no-default-features) ; LABEL="--no-default-features" ;;
    *)         ARGS=(--no-default-features --features "$combo")
               LABEL="--no-default-features --features $combo" ;;
  esac

  step "Symbol parity + tests [$LABEL]"

  # Build the cdylib for this combo into its own target dir, then diff symbols.
  OUT="target/parity-$(echo "$combo" | tr ',' '_')"
  cargo build --release --offline --lib "${ARGS[@]}" --target-dir "$OUT" \
    >/dev/null 2>&1 \
    || cargo build --release --lib "${ARGS[@]}" --target-dir "$OUT" >/dev/null \
    || { echo "cargo build FAILED for [$LABEL]"; FAIL=1; continue; }
  R_SO="$OUT/release/libdriver.so"

  diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort) \
       <(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort) \
    && echo "symbol diff: EMPTY (parity OK)" \
    || { echo "SYMBOL PARITY FAILED for [$LABEL]"; FAIL=1; }

  # Any undefined symbol that is not libc / libgcc unwinder is a real gap.
  MISSING=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' \
            | sed 's/@.*//' \
            | grep -vE '^(_ITM_|__cxa_|__gmon_start__|_Unwind_|__tls_get_addr|__errno_location)' \
            | grep -vxE 'malloc|free|calloc|realloc|posix_memalign|memcpy|memmove|memset|bcmp|strlen|strtod|abort|getenv|getcwd|readlink|realpath|open64|close|read|write|writev|lseek64|fstat64|stat64|statx|mmap64|munmap|dl_iterate_phdr|syscall|gettid|pthread_key_create|pthread_key_delete|pthread_setspecific|pthread_getspecific' \
            || true)
  if [ -n "$MISSING" ]; then
    echo "UNRESOLVED NON-LIBC SYMBOLS for [$LABEL]:"; echo "$MISSING"; FAIL=1
  else
    echo "undefined symbols: libc/unwinder only (0 missing non-libc)"
  fi

  ( cargo test --release --offline "${ARGS[@]}" 2>&1 \
    || cargo test --release "${ARGS[@]}" 2>&1 ) | tee "$OUT/test.log" \
    | grep -E "test result|DIVERGENCE \[|^error"
  if grep -qE "test result: FAILED|^error" "$OUT/test.log"; then
    echo "TESTS FAILED for [$LABEL]"; FAIL=1
  fi
done

# ------------------------------------------------------- binary/driver check
step "Binary (driver executable) check"
if grep -qE '^\s*add_executable' "$ROOT/c_src/CMakeLists.txt"; then
  echo "C builds an executable -- stdout comparison required (see below)."
  FAIL=1
else
  echo "c_src/CMakeLists.txt declares no add_executable(); the project builds"
  echo "only a shared library, so there is no binary stdout to compare."
fi
if [ -d src/bin ] || grep -qE '^\[\[bin\]\]' Cargo.toml; then
  echo "Rust declares a binary target but C does not -- investigate."; FAIL=1
else
  echo "Rust declares no [[bin]] target either. Consistent."
fi

step "SUMMARY"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$FAIL"

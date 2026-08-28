#!/usr/bin/env bash
# Full verification run: symbol parity + Phases B/C under every feature
# combination and both cargo profiles.
#
# `--offline` is required in this sandbox (crates.io index unreachable);
# libloading 0.8.9 / cfg-if 1.0.4 are in the local registry cache.
set -uo pipefail

cd "$(dirname "$0")"
CRATE_DIR="$PWD"
C_BUILD="$CRATE_DIR/../c_src/build"
fail=0

say() { printf '\n============ %s ============\n' "$*"; }

# ---------------------------------------------------------------------------
say "0. build the C reference shared object"
( cd "$CRATE_DIR/../c_src" \
  && mkdir -p build \
  && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

C_SO=$(find "$C_BUILD" -maxdepth 1 -name 'lib*.so' -type f | head -1)
[ -n "$C_SO" ] || { echo "no C .so found in $C_BUILD"; exit 1; }
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
# Feature combinations. The crate declares no [features], so the combination
# set is the three equivalent invocations below. Enumerated from Cargo.toml
# rather than hard-coded, so this keeps working if features are ever added.
FEATURE_NAMES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}
' Cargo.toml | grep -v '^default$' | tr '\n' ' ')

echo "declared non-default features: [${FEATURE_NAMES:-none}]"

COMBOS=()
COMBOS+=("")                                # default
COMBOS+=("--no-default-features")
COMBOS+=("--all-features")
if [ -n "${FEATURE_NAMES// /}" ]; then
  for f in $FEATURE_NAMES; do
    COMBOS+=("--no-default-features --features $f")
  done
  # pairwise combinations
  set -- $FEATURE_NAMES
  for a in $FEATURE_NAMES; do
    for b in $FEATURE_NAMES; do
      [ "$a" \< "$b" ] && COMBOS+=("--no-default-features --features $a,$b")
    done
  done
fi

# ---------------------------------------------------------------------------
for profile in dev release; do
  if [ "$profile" = release ]; then PROF_FLAG="--release"; PROF_DIR=release
  else PROF_FLAG=""; PROF_DIR=debug; fi

  for combo in "${COMBOS[@]}"; do
    label="profile=$profile features=[${combo:-default}]"
    say "$label"

    # shellcheck disable=SC2086
    cargo build --offline --lib $PROF_FLAG $combo >/dev/null 2>&1 || {
      echo "BUILD FAILED: $label"; fail=1; continue; }

    R_SO="$CRATE_DIR/target/$PROF_DIR/libdiv_euclid_lib.so"
    [ -f "$R_SO" ] || { echo "missing $R_SO"; fail=1; continue; }

    # --- symbol diff: every C symbol must be exported by Rust -------------
    c_syms=$(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u)
    r_syms=$(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u)
    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
    if [ -n "$missing" ]; then
      echo "SYMBOL DIFF NOT EMPTY -- Rust is missing:"; echo "$missing"; fail=1
    else
      echo "symbol diff: EMPTY ($(echo "$c_syms" | wc -l) C symbol(s) all present in Rust)"
    fi

    # --- non-libc undefined symbols in the Rust .so ------------------------
    undef=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sed 's/@.*//' \
      | grep -vE '^(_Unwind_|__cxa_|__libc_|__tls_|__errno|__gmon_start__|_ITM_|pthread_)' \
      | grep -vxE 'abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcmp|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev|sigaction|sigaltstack|sysconf|environ|__environ|dlsym|dladdr|poll|getrandom' \
      | grep -v '^$')
    if [ -n "$undef" ]; then
      echo "UNEXPECTED UNDEFINED SYMBOLS (possible untranslated C):"; echo "$undef"; fail=1
    else
      echo "undefined symbols: libc/unwind only"
    fi

    # --- Phases B, C, D ---------------------------------------------------
    # Capture to a temp file and key off cargo's real exit status (not a
    # pipeline's), so a failing test can never be reported as a pass.
    log=$(mktemp "${TMPDIR:-/tmp}/rt.XXXXXX") || { echo "mktemp failed"; exit 1; }
    # shellcheck disable=SC2086
    timeout 600 cargo test --offline $PROF_FLAG $combo >"$log" 2>&1
    rc=$?
    grep -E 'test result|FAILED|panicked|^error' "$log" || true
    if [ "$rc" -ne 0 ]; then
      echo "TESTS FAILED (exit $rc): $label"
      tail -40 "$log"
      fail=1
    elif grep -q 'FAILED\|error\[' "$log"; then
      echo "TESTS FAILED (pattern match): $label"; fail=1
    else
      # Sanity: the suite must actually have run the expected test counts.
      passed=$(grep -oE '[0-9]+ passed' "$log" | awk '{s+=$1} END {print s+0}')
      if [ "$passed" -lt 55 ]; then
        echo "SUITE TOO SMALL: only $passed tests passed in $label"; fail=1
      else
        echo "tests: $passed passed"
      fi
    fi
    rm -f "$log"
  done
done

say "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"

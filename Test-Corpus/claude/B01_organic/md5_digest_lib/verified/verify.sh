#!/usr/bin/env bash
# Full verification matrix: builds the C .so, enumerates every Cargo feature
# combination, cargo-checks each, builds the Rust .so for each, diffs exported
# symbols, and runs the differential test suites.
#
# Usage: ./verify.sh [--fast]        (--fast skips the release-profile pass)
set -uo pipefail
cd "$(dirname "$0")" || exit 1
FAST=${1:-}
FAILED=0
step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [ok]   %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------- C reference
step "Build C reference (.so), default CMake configuration"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO=$(ls c_src/build/lib*.so | head -1)
ok "C .so = $C_SO"

# ------------------------------------------------- feature-combination matrix
step "Enumerate feature combinations from Cargo.toml"
# All features declared in [features] (excluding the implicit "default").
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /=/      {split($0,a,"="); gsub(/[ \t"]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}
' Cargo.toml | sort -u)
if [ -z "$FEATURES" ]; then
  echo "  no [features] table -> exactly one configuration (the empty set)"
  COMBOS=("")
else
  echo "  features: $FEATURES"
  # Power set of the declared features.
  mapfile -t FARR <<<"$FEATURES"
  n=${#FARR[@]}
  COMBOS=()
  for ((mask=0; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FARR[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "  ${#COMBOS[@]} combination(s) to verify"

PROFILES=(debug)
[ "$FAST" = "--fast" ] || PROFILES=(debug release)

for combo in "${COMBOS[@]}"; do
  FLAGS=(--offline --no-default-features)
  [ -n "$combo" ] && FLAGS+=(--features "$combo")
  LABEL="features=[${combo:-<none>}]"

  step "cargo check  $LABEL"
  if timeout 300 cargo check "${FLAGS[@]}" --all-targets >/dev/null 2>&1; then
    ok "cargo check $LABEL"
  else
    bad "cargo check $LABEL"; timeout 300 cargo check "${FLAGS[@]}" --all-targets 2>&1 | tail -20
  fi

  for prof in "${PROFILES[@]}"; do
    PF=("${FLAGS[@]}")
    [ "$prof" = release ] && PF+=(--release)

    step "build + symbol parity  $LABEL profile=$prof"
    if timeout 300 cargo build "${PF[@]}" >/dev/null 2>&1; then
      R_SO="target/$prof/libmd5_digest_lib.so"
      if [ -f "$R_SO" ]; then
        MISSING=$(comm -23 \
          <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort) \
          <(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort))
        if [ -z "$MISSING" ]; then
          ok "symbol parity ($(nm -D --defined-only "$C_SO" | wc -l) C symbol(s), 0 missing)"
        else
          bad "symbols missing from $R_SO:"; echo "$MISSING" | sed 's/^/         /'
        fi
        NONLIBC=$(nm -D -u "$R_SO" | awk '{print $NF}' | grep -vE \
          '^(_ITM_|_Unwind_|__cxa_|__errno_location|__gmon_start__|__tls_get_addr|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev)' )
        [ -z "$NONLIBC" ] && ok "no non-libc undefined symbols" \
                          || { bad "non-libc undefined symbols:"; echo "$NONLIBC" | sed 's/^/         /'; }
      else
        bad "$R_SO not produced"
      fi
    else
      bad "cargo build $LABEL profile=$prof"
    fi

    step "differential tests  $LABEL profile=$prof"
    LOG="${TMPDIR:-/tmp}/verify-test-$$.log"
    timeout 600 cargo test "${PF[@]}" >"$LOG" 2>&1
    RC=$?
    grep -E '^test result:' "$LOG" | sed 's/^/  /'
    NRES=$(grep -cE '^test result:' "$LOG")
    if [ "$RC" -ne 0 ] || grep -qE '^test result: FAILED' "$LOG" || [ "$NRES" -lt 3 ]; then
      bad "tests $LABEL profile=$prof (exit=$RC, result lines=$NRES)"
      grep -E '^(failures:|---- |thread .* panicked|assertion)' "$LOG" | head -40
    else
      ok "tests $LABEL profile=$prof ($NRES test binaries, all green)"
    fi
    rm -f "$LOG"
  done
done

step "SUMMARY"
if [ "$FAILED" = 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAILED"

#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination + both profiles.
#
# Usage:  ./run_all_configs.sh
# Exits non-zero if the symbol diff is non-empty or any suite fails.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0
LOG="${TMPDIR:-/tmp}/verify-$$"
mkdir -p "$LOG"

hr() { printf '=%.0s' {1..72}; echo; }

# ---------------------------------------------------------------------------
# 0. Build the C shared library (never modified; rebuilt only if absent).
# ---------------------------------------------------------------------------
hr; echo "0. C shared library"
if ! ls "$ROOT"/c_src/build/lib*.so >/dev/null 2>&1; then
  ( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >"$LOG/cmake.log" 2>&1 \
    && cmake --build . >>"$LOG/cmake.log" 2>&1 ) \
    || { echo "  C build FAILED (see $LOG/cmake.log)"; exit 1; }
fi
C_SO="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
echo "  C  .so: $C_SO"

# ---------------------------------------------------------------------------
# 1. Enumerate the feature combinations declared in Cargo.toml.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/{print $1}' Cargo.toml
)
echo "  declared [features]: ${#FEATURES[@]} (${FEATURES[*]:-none})"

# The complete combination set. With no declared features this collapses to the
# three ways cargo can be asked for the one and only configuration.
COMBOS=("--all-features" "--no-default-features" "")
for f in "${FEATURES[@]:-}"; do
  [ -n "$f" ] && COMBOS+=("--no-default-features --features $f")
done

# ---------------------------------------------------------------------------
# 2. Symbol parity, per profile.
# ---------------------------------------------------------------------------
check_symbols() {
  local profile="$1" rust_so="target/$profile/liboverunder_lib.so"
  [ -f "$rust_so" ] || { echo "  MISSING $rust_so"; FAIL=1; return; }
  local c_syms="$LOG/c.syms" r_syms="$LOG/r.syms"
  nm -D --defined-only "$C_SO"   | awk '{print $3}' | sort -u > "$c_syms"
  nm -D --defined-only "$rust_so" | awk '{print $3}' | sort -u > "$r_syms"
  local missing
  missing="$(comm -23 "$c_syms" "$r_syms")"
  if [ -n "$missing" ]; then
    echo "  [$profile] MISSING FROM RUST .so:"; echo "$missing" | sed 's/^/    /'
    FAIL=1
  else
    echo "  [$profile] symbol diff EMPTY — all $(wc -l < "$c_syms") C symbols exported by Rust"
  fi
  # No stubs / unresolved non-libc imports.
  local undef
  undef="$(nm -D --undefined-only "$rust_so" | awk '{print $2}' | sed 's/@.*//' \
           | grep -vE '^(_ITM_|_Unwind_|__cxa_|__gmon_|__errno_|__tls_get_addr|_+[a-z])' \
           | grep -vxE 'abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|printf|pthread_key_create|pthread_key_delete|pthread_setspecific|putchar|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev' || true)"
  if [ -n "$undef" ]; then
    echo "  [$profile] UNEXPECTED non-libc undefined symbols:"; echo "$undef" | sed 's/^/    /'
    FAIL=1
  else
    echo "  [$profile] 0 missing/undefined non-libc symbols"
  fi
}

# ---------------------------------------------------------------------------
# 3. Run every suite under every combination, in both profiles.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  for profile in debug release; do
    relflag=""; [ "$profile" = release ] && relflag="--release"
    hr
    echo "features: $label   profile: $profile"
    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $relflag $combo > "$LOG/build.log" 2>&1; then
      echo "  BUILD FAILED"; tail -20 "$LOG/build.log"; FAIL=1; continue
    fi
    check_symbols "$profile"
    for suite in phase_b_valid phase_c_errors; do
      # shellcheck disable=SC2086
      if timeout 600 cargo test $relflag $combo --test "$suite" \
           > "$LOG/$suite.log" 2>&1; then
        echo "  $suite: $(grep -oE '[0-9]+ passed; [0-9]+ failed' "$LOG/$suite.log" | tail -1)"
      else
        echo "  $suite: FAILED"
        grep -E 'FAILED|panicked|DIVERGENCE|^ +[a-z0-9_]+$' "$LOG/$suite.log" | head -40
        FAIL=1
      fi
    done
  done
done

hr
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES DETECTED"
fi
exit "$FAIL"

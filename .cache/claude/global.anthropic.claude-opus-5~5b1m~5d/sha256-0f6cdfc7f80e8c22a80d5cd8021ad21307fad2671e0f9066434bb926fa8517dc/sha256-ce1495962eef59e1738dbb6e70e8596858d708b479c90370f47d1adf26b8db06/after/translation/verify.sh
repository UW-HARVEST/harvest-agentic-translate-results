#!/usr/bin/env bash
# Phase D driver: symbol parity + the full feature/profile matrix.
#
# Usage:  ./verify.sh
#
# 1. (Re)builds the C shared library.
# 2. Enumerates every feature combination declared in Cargo.toml (this crate declares
#    none, so the set is: default, --no-default-features, --all-features).
# 3. For every (feature combo x profile) pair: builds the Rust cdylib, diffs `nm -D`
#    against the C .so, and runs the whole differential test suite against THAT .so.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$PWD"
C_BUILD="$ROOT/../c_src/build"
FAILED=0
SUMMARY=()

note() { printf '\n\033[1m== %s ==\033[0m\n' "$*"; }
fail() { printf '\033[31mFAIL\033[0m %s\n' "$*"; FAILED=1; }
pass() { printf '\033[32mok\033[0m   %s\n' "$*"; }

# --------------------------------------------------------------------------
note "Building the C shared library"
mkdir -p "$C_BUILD" || exit 1
( cd "$C_BUILD" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null ) \
  || { fail "C build"; exit 1; }

C_SO=$(find "$C_BUILD" -maxdepth 1 -name '*.so' | head -1)
[ -n "$C_SO" ] || { fail "no C .so produced"; exit 1; }
pass "C .so: $C_SO"

# --------------------------------------------------------------------------
# Feature combinations. Extract any [features] keys from Cargo.toml; if there are
# none, the matrix is just the three canonical flag sets.
note "Enumerating feature combinations"
FEATURE_KEYS=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {sub(/[[:space:]]*=.*/,""); print}
' Cargo.toml)

COMBOS=()
if [ -z "$FEATURE_KEYS" ]; then
  echo "Cargo.toml declares no [features] -> single build configuration."
  COMBOS=("" "--no-default-features" "--all-features")
else
  # Full power set of declared features, plus the canonical flag sets.
  mapfile -t KEYS <<< "$FEATURE_KEYS"
  n=${#KEYS[@]}
  COMBOS=("" "--no-default-features" "--all-features")
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${KEYS[$i]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
fi
printf 'combinations: %s\n' "${#COMBOS[@]}"

# --------------------------------------------------------------------------
C_SYMS=$(mktemp); R_SYMS=$(mktemp)
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > "$C_SYMS"
echo "C .so defines $(wc -l < "$C_SYMS") symbols"

for PROFILE in debug release; do
  if [ "$PROFILE" = release ]; then PFLAG="--release"; else PFLAG=""; fi

  for COMBO in "${COMBOS[@]}"; do
    LABEL="profile=$PROFILE features=[${COMBO:-default}]"
    note "$LABEL"

    # shellcheck disable=SC2086
    if ! cargo build $PFLAG $COMBO > "$ROOT/build-$PROFILE.log" 2>&1; then
      fail "cargo build ($LABEL)"; SUMMARY+=("BUILD FAIL  $LABEL"); continue
    fi

    RUST_SO="$ROOT/target/$PROFILE/libfindrep_lib.so"
    if [ ! -f "$RUST_SO" ]; then
      fail "missing $RUST_SO ($LABEL)"; SUMMARY+=("NO .so      $LABEL"); continue
    fi

    # ---- symbol parity ------------------------------------------------
    nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u > "$R_SYMS"
    MISSING=$(comm -23 "$C_SYMS" "$R_SYMS")
    if [ -n "$MISSING" ]; then
      fail "symbols exported by C but NOT by Rust ($LABEL):"
      echo "$MISSING" | sed 's/^/      /'
      SUMMARY+=("SYMBOL GAP  $LABEL")
      continue
    fi
    pass "symbol parity: 0 missing (C defines $(wc -l < "$C_SYMS"))"

    # ---- undefined non-libc symbols ----------------------------------
    UNDEF=$(nm -D --undefined-only "$RUST_SO" | awk '{print $2}' \
      | grep -vE '^(__|_ITM|_Unwind|memcpy$|memset$|memcmp$|memmove$|malloc$|free$|realloc$|calloc$|abort$|raise$|write$|strlen$|dl_|pthread_|gnu_get_libc_version$|posix_memalign$|sigaltstack$|sigaction$|sigaddset$|sigemptyset$|syscall$|getenv$|bcmp$|munmap$|mmap$|mprotect$|open64$|close$|read$|readlink$|stat64$|_exit$|environ$)' || true)
    if [ -n "$UNDEF" ]; then
      echo "  note: unresolved-at-link symbols (expected to come from libc/libstd):"
      echo "$UNDEF" | sed 's/^/      /'
    fi

    # ---- differential test suite against THIS .so --------------------
    # shellcheck disable=SC2086
    if C_SO="$C_SO" RUST_SO="$RUST_SO" timeout 600 cargo test $PFLAG $COMBO > "$ROOT/test-$PROFILE-$(echo "${COMBO:-default}" | tr -c 'A-Za-z0-9' '_').log" 2>&1; then
      COUNTS=$(grep -h '^test result:' "$ROOT/test-$PROFILE-$(echo "${COMBO:-default}" | tr -c 'A-Za-z0-9' '_').log" \
        | awk '{p+=$4; f+=$6} END {printf "%d passed, %d failed", p, f}')
      pass "tests: $COUNTS"
      SUMMARY+=("OK          $LABEL  ($COUNTS)")
    else
      fail "test suite ($LABEL) — see the log"
      grep -hE '^(test result:|---- |failures:)' \
        "$ROOT/test-$PROFILE-$(echo "${COMBO:-default}" | tr -c 'A-Za-z0-9' '_').log" | head -30
      SUMMARY+=("TEST FAIL   $LABEL")
    fi
  done
done

rm -f "$C_SYMS" "$R_SYMS"

note "SUMMARY"
for s in "${SUMMARY[@]}"; do echo "  $s"; done
if [ "$FAILED" -eq 0 ]; then
  printf '\n\033[32mALL CONFIGURATIONS PASS\033[0m\n'
else
  printf '\n\033[31mSOME CONFIGURATIONS FAILED\033[0m\n'
fi
exit "$FAILED"

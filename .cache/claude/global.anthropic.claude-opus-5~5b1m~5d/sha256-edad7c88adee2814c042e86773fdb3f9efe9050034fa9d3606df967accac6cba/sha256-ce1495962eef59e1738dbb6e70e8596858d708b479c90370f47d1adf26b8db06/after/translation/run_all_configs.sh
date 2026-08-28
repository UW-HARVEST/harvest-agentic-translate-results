#!/usr/bin/env bash
# Phase D driver: symbol parity + the full differential suite across EVERY
# feature combination and EVERY build profile.
#
# Usage:  cd translation && ./run_all_configs.sh
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
CARGO="cargo --offline"
FAIL=0

hdr() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
ok()  { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad() { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
# 0. Build the C shared library (the ground truth)
# ---------------------------------------------------------------------------
hdr "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)"
[ -n "$C_SO" ] || { bad "no C .so produced"; exit 1; }
ok "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 1. Enumerate every feature combination declared in Cargo.toml
# ---------------------------------------------------------------------------
hdr "Enumerating feature combinations"
FEATURES=$(cargo metadata --offline --no-deps --format-version 1 2>/dev/null \
  | python3 -c '
import json,sys
d=json.load(sys.stdin)
f=[k for k in d["packages"][0]["features"] if k!="default"]
print(" ".join(f))')

# Build the list of combos to test: the default build, then --no-default-features
# with the powerset of the declared features.
COMBOS=("default")
if [ -n "${FEATURES// /}" ]; then
  # shellcheck disable=SC2206
  FARR=($FEATURES)
  n=${#FARR[@]}
  total=$(( 1 << n ))
  for (( mask=0; mask<total; mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FARR[$i]}"; fi
    done
    COMBOS+=("nodefault:${combo}")
  done
  ok "declared features: $FEATURES  (${#COMBOS[@]} combinations)"
else
  ok "the crate declares NO cargo features - the only configuration is the"
  printf '       default build, so 1 combination covers the whole crate.\n'
fi

# ---------------------------------------------------------------------------
# 2. For each combo x profile: build, diff nm -D, run the suite
# ---------------------------------------------------------------------------
nm_names() { nm -D --defined-only "$1" | awk '{print $3}' | sort; }

C_NAMES=$(mktemp); nm_names "$C_SO" > "$C_NAMES"
C_COUNT=$(wc -l < "$C_NAMES")

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    default)          FLAGS=() ; label="default features" ;;
    nodefault:)       FLAGS=(--no-default-features) ; label="--no-default-features" ;;
    nodefault:*)      FLAGS=(--no-default-features --features "${combo#nodefault:}")
                      label="--no-default-features --features ${combo#nodefault:}" ;;
  esac

  for profile in dev release; do
    if [ "$profile" = release ]; then PFLAGS=(--release); PDIR=release
    else PFLAGS=(); PDIR=debug; fi

    hdr "combo: $label   profile: $profile"

    $CARGO check "${FLAGS[@]}" "${PFLAGS[@]}" --all-targets >/dev/null 2>&1 \
      && ok "cargo check" || bad "cargo check"

    $CARGO build "${FLAGS[@]}" "${PFLAGS[@]}" >/dev/null 2>&1 \
      && ok "cargo build" || bad "cargo build"

    RS_SO="target/$PDIR/libstr_dups_lib.so"
    if [ ! -f "$RS_SO" ]; then bad "missing $RS_SO"; continue; fi

    # --- symbol parity -----------------------------------------------------
    RS_NAMES=$(mktemp); nm_names "$RS_SO" > "$RS_NAMES"
    MISSING=$(comm -23 "$C_NAMES" "$RS_NAMES")
    EXTRA=$(comm -13 "$C_NAMES" "$RS_NAMES")
    if [ -z "$MISSING" ]; then
      ok "nm -D: all $C_COUNT C symbols exported by the Rust .so"
    else
      bad "nm -D: MISSING from Rust .so:"; echo "$MISSING" | sed 's/^/        /'
    fi
    if [ -n "$EXTRA" ]; then
      bad "nm -D: EXTRA public symbols in the Rust .so:"; echo "$EXTRA" | sed 's/^/        /'
    else
      ok "nm -D: no extra public symbols"
    fi
    # undefined non-libc / non-libgcc symbols
    UNDEF=$(nm -D --undefined-only "$RS_SO" \
      | awk '$1=="U"{print $2}' \
      | sed 's/@.*//' \
      | grep -vE '^(_Unwind_|__errno_location|__tls_get_addr|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat|fstat64|getcwd|getenv|lseek|lseek64|malloc|memcmp|memcpy|memmove|memset|mmap|mmap64|munmap|open|open64|posix_memalign|printf|pthread_|read|readlink|realloc|realpath|sprintf|stat|stat64|strcmp|strlen|syscall|write|writev|gettid|statx|__cxa_)' \
      || true)
    if [ -z "$UNDEF" ]; then
      ok "nm -D: 0 undefined non-libc symbols"
    else
      bad "nm -D: undefined non-libc symbols:"; echo "$UNDEF" | sed 's/^/        /'
    fi
    rm -f "$RS_NAMES"

    # --- the differential suite, against THIS .so --------------------------
    # (cargo test's own harness always builds with the dev profile; the
    #  RUST_SO override is what selects which cdylib is actually loaded.)
    LOG=$(mktemp)
    if RUST_SO="$PWD/$RS_SO" timeout 600 $CARGO test "${FLAGS[@]}" \
      --test smoke --test phase_b_low --test phase_b_map --test phase_b_string \
      --test phase_b_top --test phase_c_errors \
      -- --test-threads=1 > "$LOG" 2>&1; then
      PASSED=$(grep -Eo '^test result: ok\. [0-9]+' "$LOG" | awk '{s+=$4} END{print s}')
      ok "differential suite vs $RS_SO: $PASSED tests passed"
    else
      bad "differential suite vs $RS_SO"
      grep -E '^(test .*FAILED|failures:|thread .* panicked|DIVERGENCE)' "$LOG" | head -40 | sed 's/^/        /'
    fi
    rm -f "$LOG"
  done
done

rm -f "$C_NAMES"

hdr "RESULT"
if [ "$FAIL" -eq 0 ]; then
  printf '\033[32mALL CONFIGURATIONS PASS\033[0m\n'
else
  printf '\033[31mFAILURES PRESENT\033[0m\n'
fi
exit "$FAIL"

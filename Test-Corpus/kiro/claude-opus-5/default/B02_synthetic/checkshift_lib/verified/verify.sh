#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination + every profile.
#
#   ./verify.sh
#
# Must be run from the crate root (translation/). Tests are run with
# --test-threads=1 because the harness redirects fd 1 process-wide to capture
# each library's printf output; parallel tests would interleave into the
# captured buffers.
set -uo pipefail

cd "$(dirname "$0")"
ROOT=".."
FAIL=0
step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
step "Build the C shared library"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/*.so | head -1)
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
step "Enumerate feature combinations"
# Mechanically extract features from Cargo.toml (no hand-written list).
FEATURES=$(python3 - "$PWD/Cargo.toml" <<'PY'
import re, sys
txt = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
if not m:
    print("")
else:
    names = re.findall(r'^\s*([A-Za-z0-9_-]+)\s*=', m.group(1), re.M)
    print(" ".join(n for n in names if n != "default"))
PY
)
if [ -z "$FEATURES" ]; then
    echo "no [features] declared -> single configuration (default)"
    COMBOS=("DEFAULT")
else
    echo "features: $FEATURES"
    COMBOS=("DEFAULT" "NONE")
    for f in $FEATURES; do COMBOS+=("$f"); done
    ALL=$(echo "$FEATURES" | tr ' ' ',')
    COMBOS+=("$ALL")
fi

# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  for profile in release debug; do
    case "$combo" in
      DEFAULT) FEATFLAGS=() ;;
      NONE)    FEATFLAGS=(--no-default-features) ;;
      *)       FEATFLAGS=(--no-default-features --features "$combo") ;;
    esac
    case "$profile" in
      release) PROFFLAGS=(--release) ;;
      debug)   PROFFLAGS=() ;;
    esac

    step "combo=$combo profile=$profile"

    if ! timeout 600 cargo build "${PROFFLAGS[@]}" "${FEATFLAGS[@]}" >/dev/null 2>&1; then
        echo "  cargo build FAILED"; FAIL=1; continue
    fi
    R_SO="target/$profile/libcheckshift_lib.so"
    if [ ! -f "$R_SO" ]; then echo "  missing $R_SO"; FAIL=1; continue; fi

    # ---- symbol parity ----
    nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}' | sort > /tmp/c_syms.$$
    nm -D --defined-only "$R_SO" | awk '$2=="T"{print $3}' | sort > /tmp/r_syms.$$
    MISSING=$(comm -23 /tmp/c_syms.$$ /tmp/r_syms.$$)
    if [ -n "$MISSING" ]; then
        echo "  SYMBOL PARITY FAILED — missing from Rust .so:"; echo "$MISSING" | sed 's/^/    /'
        FAIL=1
    else
        echo "  symbol parity: OK ($(wc -l < /tmp/c_syms.$$) symbols, 0 missing)"
    fi
    # undefined non-libc / non-unwind symbols
    UNDEF=$(nm -D --undefined-only "$R_SO" \
        | awk '{print $NF}' \
        | grep -vE '@GLIBC|@GCC|^_ITM_|^__cxa_|^__gmon_start__$|^_Unwind_|^gettid$|^statx$' || true)
    if [ -n "$UNDEF" ]; then
        echo "  UNDEFINED non-libc symbols:"; echo "$UNDEF" | sed 's/^/    /'; FAIL=1
    else
        echo "  undefined non-libc symbols: 0"
    fi
    rm -f /tmp/c_syms.$$ /tmp/r_syms.$$

    # ---- differential tests against THIS .so ----
    OUT=$(CHECKSHIFT_RUST_SO="$PWD/$R_SO" \
          timeout 600 cargo test "${PROFFLAGS[@]}" "${FEATFLAGS[@]}" \
          -- --test-threads=1 2>&1)
    echo "$OUT" | grep -E '^test result:' | sed 's/^/  /'
    if echo "$OUT" | grep -qE '^test result: FAILED|error\[|error:'; then
        echo "$OUT" | grep -E '^(---- |thread )|panicked|FAILED' | head -20 | sed 's/^/    /'
        FAIL=1
    fi
  done
done

step "SUMMARY"
if [ "$FAIL" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"

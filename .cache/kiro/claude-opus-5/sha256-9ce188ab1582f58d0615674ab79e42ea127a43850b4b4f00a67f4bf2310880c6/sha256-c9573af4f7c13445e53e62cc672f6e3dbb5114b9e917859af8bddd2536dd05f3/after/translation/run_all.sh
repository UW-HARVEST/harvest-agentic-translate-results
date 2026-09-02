#!/usr/bin/env bash
# Phase D driver: rebuild both objects, check dynamic-symbol parity, and run the
# whole differential suite under every feature combination and both profiles.
#
# Usage: ./run_all.sh        (from the translation/ directory)
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
C_SO="$ROOT/c_src/build/libdriver.so"
FAIL=0

say() { printf '\n\033[1m== %s ==\033[0m\n' "$*"; }
fail() { printf '\033[31mFAIL\033[0m %s\n' "$*"; FAIL=1; }
ok() { printf '\033[32mok\033[0m   %s\n' "$*"; }

# --------------------------------------------------------------------------
say "Building the C reference shared object"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) >/tmp/cbuild.log 2>&1 \
  || { fail "cmake build"; tail -20 /tmp/cbuild.log; exit 1; }
[ -f "$C_SO" ] || { fail "missing $C_SO"; exit 1; }
ok "$C_SO"

# --------------------------------------------------------------------------
# Enumerate feature combinations declared in Cargo.toml. The crate declares
# none, so the matrix is {default, --no-default-features}; the loop is written
# generically so it keeps working if features are added later.
say "Enumerating feature combinations from Cargo.toml"
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' "$CRATE_DIR/Cargo.toml")
echo "declared non-default features: [${FEATURES:-none}]"

COMBOS=("" "--no-default-features")
if [ -n "$FEATURES" ]; then
    # Full power set of the declared features, on top of --no-default-features.
    mapfile -t FLIST <<<"$FEATURES"
    n=${#FLIST[@]}
    for ((mask = 1; mask < (1 << n); mask++)); do
        sel=""
        for ((i = 0; i < n; i++)); do
            (((mask >> i) & 1)) && sel="${sel:+$sel,}${FLIST[$i]}"
        done
        COMBOS+=("--no-default-features --features $sel")
        COMBOS+=("--features $sel")
    done
fi
printf 'combination: [%s]\n' "${COMBOS[@]}"

# --------------------------------------------------------------------------
cd "$CRATE_DIR"
for profile in release debug; do
    if [ "$profile" = release ]; then PROF_FLAG=--release; else PROF_FLAG=; fi
    for combo in "${COMBOS[@]}"; do
        label="profile=$profile combo=[${combo:-default}]"

        say "cargo check — $label"
        # shellcheck disable=SC2086
        if timeout 600 cargo check $PROF_FLAG $combo --all-targets >/tmp/check.log 2>&1
        then ok "cargo check ($label)"
        else fail "cargo check ($label)"; tail -30 /tmp/check.log; continue
        fi

        say "cargo build + symbol parity — $label"
        # shellcheck disable=SC2086
        if timeout 600 cargo build $PROF_FLAG $combo >/tmp/build.log 2>&1
        then ok "cargo build ($label)"
        else fail "cargo build ($label)"; tail -30 /tmp/build.log; continue
        fi

        RUST_SO="target/$profile/libdriver.so"
        if [ ! -f "$RUST_SO" ]; then fail "missing $RUST_SO"; continue; fi

        nm -D --defined-only "$C_SO"   | awk '$2=="T"{print $3}' | sort >/tmp/c.sym
        nm -D --defined-only "$RUST_SO" | awk '$2=="T"{print $3}' | sort >/tmp/r.sym
        MISSING=$(comm -23 /tmp/c.sym /tmp/r.sym)
        if [ -z "$MISSING" ]; then
            ok "symbol diff empty ($(wc -l </tmp/c.sym) C symbols, all present in Rust)"
        else
            fail "symbols exported by C but missing from Rust ($label):"
            echo "$MISSING"
        fi

        say "cargo test — $label"
        # shellcheck disable=SC2086
        if timeout 600 cargo test $PROF_FLAG $combo >/tmp/test.log 2>&1
        then ok "cargo test ($label): $(grep -c '^test .* ok$' /tmp/test.log) tests passed"
        else fail "cargo test ($label)"; grep -E '^(test .*FAILED|failures:|error)' -A5 /tmp/test.log | head -60
        fi
    done
done

say "SUMMARY"
if [ "$FAIL" -eq 0 ]; then
    printf '\033[32mALL CHECKS PASSED\033[0m\n'
else
    printf '\033[31mTHERE WERE FAILURES\033[0m\n'
fi
exit "$FAIL"

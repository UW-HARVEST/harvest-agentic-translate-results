#!/usr/bin/env bash
# Phase D driver: build the C .so, then run the whole differential suite under
# every feature combination × build profile, and diff `nm -D` symbol sets.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
export CARGO_NET_OFFLINE="${CARGO_NET_OFFLINE:-true}"

fail=0
note() { printf '\n=== %s ===\n' "$*"; }
bad()  { printf '!!! FAIL: %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
# 1. C shared library
# ---------------------------------------------------------------------------
note "building C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || bad "C build"
C_SO="$ROOT/c_src/build/libdriver.so"
[ -f "$C_SO" ] || bad "missing $C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations straight out of Cargo.toml
# ---------------------------------------------------------------------------
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /=/      {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}
' "$HERE/Cargo.toml")

COMBOS=()
if [ -z "$FEATURES" ]; then
    # No [features] table: the only two distinct configurations.
    COMBOS+=("--no-default-features")
    COMBOS+=("")
else
    COMBOS+=("--no-default-features")
    COMBOS+=("")
    COMBOS+=("--all-features")
    for f in $FEATURES; do
        COMBOS+=("--no-default-features --features $f")
        COMBOS+=("--features $f")
    done
fi

note "feature combinations to verify"
printf '  [%s]\n' "${COMBOS[@]}"

# ---------------------------------------------------------------------------
# 3. cargo check / build / test each combination in both profiles
# ---------------------------------------------------------------------------
for profile in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="profile='${profile:-debug}' features='${combo:-default}'"
    note "cargo check  $label"
    # shellcheck disable=SC2086
    timeout 600 cargo check --manifest-path "$HERE/Cargo.toml" --tests $profile $combo \
        >"$HERE/target/check.log" 2>&1 || { tail -30 "$HERE/target/check.log"; bad "check $label"; continue; }

    note "cargo build  $label"
    # shellcheck disable=SC2086
    timeout 600 cargo build --manifest-path "$HERE/Cargo.toml" $profile $combo \
        >"$HERE/target/build.log" 2>&1 || { tail -30 "$HERE/target/build.log"; bad "build $label"; continue; }

    # symbol parity for the .so produced by THIS configuration
    if [ -n "$profile" ]; then R_SO="$HERE/target/release/libdriver.so"; else R_SO="$HERE/target/debug/libdriver.so"; fi
    if [ ! -f "$R_SO" ]; then bad "missing $R_SO for $label"; continue; fi
    cdef=$(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u)
    rdef=$(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u)
    missing=$(comm -23 <(printf '%s\n' "$cdef") <(printf '%s\n' "$rdef"))
    extra=$(comm -13 <(printf '%s\n' "$cdef") <(printf '%s\n' "$rdef"))
    if [ -n "$missing" ]; then
        printf 'symbols in C but NOT in Rust:\n%s\n' "$missing"; bad "symbol diff $label"
    fi
    if [ -n "$extra" ]; then
        printf 'symbols in Rust but NOT in C:\n%s\n' "$extra"; bad "extra symbols $label"
    fi
    [ -z "$missing" ] && [ -z "$extra" ] && echo "  symbol diff EMPTY ($(printf '%s\n' "$cdef" | wc -l) symbols)"

    note "cargo test   $label"
    # shellcheck disable=SC2086
    if timeout 600 cargo test --manifest-path "$HERE/Cargo.toml" $profile $combo \
        >"$HERE/target/test.log" 2>&1; then
        grep -E '^test result:' "$HERE/target/test.log" | sed 's/^/  /'
    else
        tail -60 "$HERE/target/test.log"; bad "test $label"
    fi
  done
done

note "SUMMARY"
if [ "$fail" -eq 0 ]; then
    echo "ALL CONFIGURATIONS PASSED"
else
    echo "THERE WERE FAILURES"
fi
exit "$fail"

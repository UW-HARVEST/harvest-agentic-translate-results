#!/usr/bin/env bash
# Differential-test driver (Phase B + C + D).
#
# `cargo test` does NOT rebuild the `cdylib` (an integration test target does not
# depend on it), so the cdylib MUST be built explicitly before each test run --
# otherwise the tests silently `dlopen` a stale `libgen_ray_lib.so`.
set -euo pipefail
cd "$(dirname "$0")"
export CARGO_NET_OFFLINE=true
WORK=$(mktemp -d "${TMPDIR:-/tmp}/diffsyms.XXXXXX" 2>/dev/null || mktemp -d ./.diffsyms.XXXXXX)
trap 'rm -rf "$WORK"' EXIT

# ---------------------------------------------------------------- C reference
if [ ! -f ../c_src/build/CMakeCache.txt ]; then
    mkdir -p ../c_src/build
    (cd ../c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null)
fi
cmake --build ../c_src/build >/dev/null
C_SO=$(echo ../c_src/build/*.so)
echo "C reference: $C_SO"

# ------------------------------------------------------- feature combinations
# Cargo.toml declares no [features] table, so the complete set of feature
# combinations is {} == {default}. Both are run anyway, under both profiles,
# so this stays correct if features are ever added.
NFEAT=$(cargo metadata --offline --format-version 1 --no-deps \
        | tr '{},' '\n' | grep -c '^"[a-z_-]*":\["' || true)
echo "declared features in Cargo.toml: $(grep -c '^\[features\]' Cargo.toml || true) section(s)"

FAIL=0
for combo in "--no-default-features" "--all-features" ""; do
  for profile in "" "--release"; do
    echo
    echo "=============================================================="
    echo "== features: '${combo:-<default>}'   profile: '${profile:-debug}'"
    echo "=============================================================="
    # shellcheck disable=SC2086
    cargo build --offline $combo $profile 2>&1 | grep -Ev '^\s*$' | tail -3
    # shellcheck disable=SC2086
    if ! cargo test --offline $combo $profile "$@" -- --test-threads=4; then
        FAIL=1
    fi
  done
done

# --------------------------------------------------------------- Phase D gate
echo
echo "=============================================================="
echo "== Phase D: nm -D symbol parity"
echo "=============================================================="
for prof in debug release; do
    R_SO="target/$prof/libgen_ray_lib.so"
    [ -f "$R_SO" ] || continue
    nm -D --defined-only "$C_SO" | awk '{print $3}' | sort > "$WORK/c.txt"
    nm -D --defined-only "$R_SO" | awk '{print $3}' | sort > "$WORK/r.txt"
    MISSING=$(comm -23 "$WORK/c.txt" "$WORK/r.txt")
    EXTRA=$(comm -13 "$WORK/c.txt" "$WORK/r.txt")
    printf '%-8s C=%s Rust=%s  ' "$prof" "$(wc -l < "$WORK/c.txt")" "$(wc -l < "$WORK/r.txt")"
    if [ -n "$MISSING" ]; then
        echo "MISSING FROM RUST:"; echo "$MISSING"; FAIL=1
    elif [ -n "$EXTRA" ]; then
        echo "EXTRA IN RUST: $(echo "$EXTRA" | tr '\n' ' ')"; FAIL=1
    else
        echo "parity OK (0 missing, 0 extra)"
    fi
    # No undefined NON-libc symbols. Everything the Rust cdylib imports must be
    # versioned against GLIBC (or be a linker-provided `_`-prefixed symbol).
    UND=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' \
          | grep -vE '@GLIBC_' \
          | grep -vE '^(_ITM_|__|_Unwind_|__gmon)' || true)
    if [ -n "$UND" ]; then
        echo "  UNDEFINED NON-LIBC in $prof:"; echo "$UND"; FAIL=1
    else
        echo "         $prof: 0 undefined non-libc symbols ($(nm -D --undefined-only "$R_SO" | grep -c '@GLIBC_') libc imports)"
    fi
done

echo
if [ "$FAIL" -eq 0 ]; then
    echo "ALL DIFFERENTIAL TESTS + SYMBOL PARITY PASSED"
else
    echo "FAILURES DETECTED"; exit 1
fi

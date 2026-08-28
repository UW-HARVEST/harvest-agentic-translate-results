#!/usr/bin/env bash
# Differential test driver.
#
# `cargo test` does not build `cdylib` artifacts, so the shared objects the tests
# load via `libloading` have to be produced with explicit `cargo build` runs --
# once per profile, so both the unoptimised and the optimised code generation get
# compared against C.
#
# `Cargo.toml` declares no `[features]`, so there is exactly one feature
# combination (the empty one); it is still spelled out explicitly below in the
# form the task asks for, and re-derived from `Cargo.toml` so that adding a
# feature later makes this script cover it.
set -uo pipefail

cd "$(dirname "$0")"
LOG=/tmp/match-verify.log
: >"$LOG"
status=0

run() {
    echo "### $*" | tee -a "$LOG"
    if ! timeout 600 "$@" >>"$LOG" 2>&1; then
        echo "    FAILED: $*"
        status=1
    fi
}

# --- enumerate feature combinations ------------------------------------------
mapfile -t FEATURES < <(
    awk '
        /^\[features\]/ { inside = 1; next }
        /^\[/           { inside = 0 }
        inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            sub(/[[:space:]]*=.*/, "");
            if ($0 != "default") print
        }
    ' Cargo.toml
)

COMBOS=("")
for f in "${FEATURES[@]:-}"; do
    [ -n "$f" ] || continue
    for existing in "${COMBOS[@]}"; do
        COMBOS+=("${existing:+$existing,}$f")
    done
done

echo "feature combinations: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do
    echo "  - '${c:-<none>}'"
done

# --- C reference library -----------------------------------------------------
if ! ls ../c_src/build/lib*.so >/dev/null 2>&1; then
    echo "### building C reference" | tee -a "$LOG"
    (
        cd ../c_src && mkdir -p build && cd build &&
            cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
    ) >>"$LOG" 2>&1 || {
        echo "    FAILED: C build"
        exit 1
    }
fi

# --- check, build and test every combination --------------------------------
for combo in "${COMBOS[@]}"; do
    echo
    echo "=== features: '${combo:-<none>}' ==="
    args=(--no-default-features)
    [ -n "$combo" ] && args+=(--features "$combo")

    run cargo check "${args[@]}" --all-targets
    run cargo build "${args[@]}"           # target/debug/*.so
    run cargo build "${args[@]}" --release # target/release/*.so
    run cargo test "${args[@]}"

    # Export-parity check: every symbol the C library exports must be exported
    # by the Rust library under the exact same name.
    for profile in debug release; do
        rust_so=$(ls target/$profile/lib*.so 2>/dev/null | head -n1)
        [ -n "$rust_so" ] || continue
        c_so=$(ls ../c_src/build/lib*.so | head -n1)
        missing=$(comm -23 \
            <(nm -D --defined-only "$c_so" | awk '$2 ~ /^[TWDBR]$/ {print $3}' | sort -u) \
            <(nm -D --defined-only "$rust_so" | awk '$2 ~ /^[TWDBR]$/ {print $3}' | sort -u))
        if [ -n "$missing" ]; then
            echo "    FAILED: $profile is missing exports:"
            echo "$missing" | sed 's/^/      /'
            status=1
        else
            echo "    exports OK ($profile)"
        fi
    done
done

echo
if [ "$status" -eq 0 ]; then
    echo "ALL COMBINATIONS PASSED"
else
    echo "FAILURES -- see $LOG"
fi
exit "$status"

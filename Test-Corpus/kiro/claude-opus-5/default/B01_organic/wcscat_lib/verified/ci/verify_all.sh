#!/usr/bin/env bash
# Phase D driver: enumerate every feature combination declared in Cargo.toml and
# run cargo check + the full differential suite for each, in both the debug and
# release profiles (release uses different codegen and `panic = "abort"`).
#
# Feature names are extracted mechanically from Cargo.toml rather than assumed.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
CRATE_DIR="$PWD"
ROOT="$(dirname "$CRATE_DIR")"

# ---------------------------------------------------------------------------
# Ensure the C ground-truth .so exists.
# ---------------------------------------------------------------------------
if ! ls "$ROOT"/c_src/build/lib*.so >/dev/null 2>&1; then
    echo "==> building the C shared library"
    ( mkdir -p "$ROOT/c_src/build" \
      && cd "$ROOT/c_src/build" \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi
echo "==> C .so: $(ls "$ROOT"/c_src/build/lib*.so)"

# ---------------------------------------------------------------------------
# Extract the feature list from the [features] table of Cargo.toml.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
    awk '
        /^\[features\]/       { in_f = 1; next }
        /^\[/                 { in_f = 0 }
        in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            split($0, a, "=");
            gsub(/[[:space:]]/, "", a[1]);
            if (a[1] != "default") print a[1];
        }
    ' Cargo.toml
)

echo "==> declared features: ${#FEATURES[@]} (${FEATURES[*]:-none})"

# Build the list of feature-flag combinations to test. With no [features] table
# the only distinct configurations are the default build and the explicit
# --no-default-features / --all-features spellings of it.
COMBOS=("" "--no-default-features" "--all-features")
if [ "${#FEATURES[@]}" -gt 0 ]; then
    n=${#FEATURES[@]}
    total=$((1 << n))
    for ((mask = 0; mask < total; mask++)); do
        sel=""
        for ((b = 0; b < n; b++)); do
            if (( (mask >> b) & 1 )); then
                sel="${sel:+$sel,}${FEATURES[b]}"
            fi
        done
        if [ -n "$sel" ]; then
            COMBOS+=("--no-default-features --features $sel")
        else
            COMBOS+=("--no-default-features")
        fi
    done
fi

# De-duplicate.
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')

echo "==> ${#COMBOS[@]} feature combination(s) to verify"

FAIL=0
for combo in "${COMBOS[@]}"; do
    label="${combo:-<default>}"
    for profile in "" "--release"; do
        plabel="${profile:-debug}"
        echo
        echo "=============================================================="
        echo "  features: $label   profile: $plabel"
        echo "=============================================================="

        # shellcheck disable=SC2086
        if ! timeout 300 cargo check $combo $profile --all-targets >/tmp/dc.log 2>&1; then
            echo "  cargo check FAILED"; tail -30 /tmp/dc.log; FAIL=1; continue
        fi
        echo "  cargo check ok"

        # shellcheck disable=SC2086
        if ! timeout 300 cargo build $combo $profile >/tmp/db.log 2>&1; then
            echo "  cargo build FAILED"; tail -30 /tmp/db.log; FAIL=1; continue
        fi

        # Symbol parity for this configuration's cdylib.
        so="target/$( [ -n "$profile" ] && echo release || echo debug )/libwcscat_lib.so"
        if [ ! -f "$so" ]; then
            echo "  MISSING $so"; FAIL=1; continue
        fi
        cso=$(ls "$ROOT"/c_src/build/lib*.so | head -1)
        missing=$(comm -23 \
            <(nm -D --defined-only "$cso" | awk '{print $NF}' | sort -u) \
            <(nm -D --defined-only "$so"  | awk '{print $NF}' | sort -u))
        if [ -n "$missing" ]; then
            echo "  SYMBOL PARITY FAILED, missing from Rust .so:"; echo "$missing"; FAIL=1
        else
            echo "  symbol parity ok (0 missing)"
        fi

        # shellcheck disable=SC2086
        if ! timeout 500 cargo test $combo $profile >/tmp/dt.log 2>&1; then
            echo "  cargo test FAILED"; grep -E 'FAILED|panicked|DIVERGENCE|test result' /tmp/dt.log | head -40; FAIL=1; continue
        fi
        grep -E 'test result' /tmp/dt.log | sed 's/^/  /'
    done
done

echo
if [ "$FAIL" -eq 0 ]; then
    echo "ALL FEATURE COMBINATIONS x PROFILES PASSED"
else
    echo "FAILURES PRESENT"
fi
exit "$FAIL"

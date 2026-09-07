#!/usr/bin/env bash
# Phase D driver: run the whole differential suite against every feature
# combination declared in Cargo.toml, for both Rust build profiles, and diff
# `nm -D` on the two shared libraries each time.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"
CSO="$(ls "$ROOT"/c_src/build/lib*.so 2>/dev/null | head -1)"
if [[ -z "$CSO" ]]; then
    echo "FATAL: C .so not built. Run:"
    echo "  cd $ROOT/c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    exit 1
fi
echo "C  .so: $CSO"

# --- enumerate feature combinations from Cargo.toml -------------------------
FEATURES=$(awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/, ""); print }
' Cargo.toml | grep -v '^default$' | sort -u)

COMBOS=()
COMBOS+=("--")                       # default features
COMBOS+=("--no-default-features")
if [[ -n "$FEATURES" ]]; then
    COMBOS+=("--all-features")
    for f in $FEATURES; do
        COMBOS+=("--no-default-features --features $f")
    done
    # pairwise
    for a in $FEATURES; do
        for b in $FEATURES; do
            [[ "$a" < "$b" ]] && COMBOS+=("--no-default-features --features $a,$b")
        done
    done
else
    echo "Cargo.toml declares no [features]; default == --no-default-features."
fi

echo "feature combinations to verify: ${#COMBOS[@]}"

FAIL=0
for profile in release debug; do
    if [[ "$profile" == "release" ]]; then PFLAG="--release"; else PFLAG=""; fi
    for combo in "${COMBOS[@]}"; do
        [[ "$combo" == "--" ]] && combo=""
        LABEL="profile=$profile features=[${combo:-default}]"
        echo "=============================================================="
        echo ">>> $LABEL"

        if ! timeout 600 cargo build $PFLAG $combo >/tmp/build.log 2>&1; then
            echo "BUILD FAILED ($LABEL)"; tail -30 /tmp/build.log; FAIL=1; continue
        fi
        RSO="target/$profile/libgjk_lib.so"
        if [[ ! -f "$RSO" ]]; then
            echo "MISSING $RSO ($LABEL)"; FAIL=1; continue
        fi

        # symbol diff
        nm -D --defined-only "$CSO" | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort > /tmp/c.syms
        nm -D --defined-only "$RSO" | awk '$2=="T"||$2=="D"||$2=="B"{print $3}' | sort > /tmp/r.syms
        MISSING=$(comm -23 /tmp/c.syms /tmp/r.syms)
        if [[ -n "$MISSING" ]]; then
            echo "SYMBOL DIFF NOT EMPTY ($LABEL):"; echo "$MISSING"; FAIL=1
        else
            echo "symbols: $(wc -l < /tmp/c.syms) C / $(wc -l < /tmp/r.syms) Rust, 0 missing"
        fi

        # full differential suite against this exact .so
        if ! GJK_C_SO="$CSO" GJK_RUST_SO="$(pwd)/$RSO" \
             timeout 600 cargo test $PFLAG $combo >/tmp/test.log 2>&1; then
            echo "TESTS FAILED ($LABEL)"
            grep -E '^(test .*FAILED|failures:)|panicked' /tmp/test.log | head -40
            FAIL=1
        else
            grep -hE '^test result:' /tmp/test.log | sed 's/^/  /'
        fi
    done
done

echo "=============================================================="
if [[ $FAIL -eq 0 ]]; then
    echo "ALL FEATURE COMBINATIONS AND PROFILES PASSED"
else
    echo "FAILURES DETECTED"
fi
exit $FAIL

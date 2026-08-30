#!/usr/bin/env bash
# Runs the whole differential suite under EVERY feature combination and under
# both build profiles, plus the raw `nm -D` symbol diff.
set -u
cd "$(dirname "$0")"
export CARGO_NET_OFFLINE=true
fail=0

# --- make sure the C reference library exists ---------------------------------
if [ ! -f ../c_src/build/libdriver.so ]; then
    (cd ../c_src && mkdir -p build && cd build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . >/dev/null)
fi

# --- enumerate feature combinations from Cargo.toml ---------------------------
FEATURES=$(python3 - <<'PY'
import re
s = open('Cargo.toml').read()
m = re.search(r'^\[features\](.*?)(^\[|\Z)', s, re.S | re.M)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.strip()
        if line and not line.startswith('#') and '=' in line:
            names.append(line.split('=')[0].strip())
print(' '.join(n for n in names if n != 'default'))
PY
)
echo "declared features: '${FEATURES}'"

combos=()
if [ -z "$FEATURES" ]; then
    echo "no [features] table -> the default (empty) feature set is the only combination"
    combos+=("DEFAULT")
else
    combos+=("DEFAULT")
    combos+=("NONE")
    # power set of the declared features
    read -r -a arr <<<"$FEATURES"
    n=${#arr[@]}
    for ((mask = 1; mask < (1 << n); mask++)); do
        sel=""
        for ((i = 0; i < n; i++)); do
            (((mask >> i) & 1)) && sel="${sel:+$sel,}${arr[i]}"
        done
        combos+=("$sel")
    done
fi

# --- run every combination under both profiles --------------------------------
for profile in debug release; do
    relflag=""
    [ "$profile" = release ] && relflag="--release"
    for combo in "${combos[@]}"; do
        case "$combo" in
        DEFAULT) fflags="" ;;
        NONE) fflags="--no-default-features" ;;
        *) fflags="--no-default-features --features $combo" ;;
        esac

        echo "=============================================================="
        echo "profile=$profile features=$combo"
        echo "=============================================================="
        # Build the cdylib for this configuration first, then point the tests at it.
        # shellcheck disable=SC2086
        if ! cargo build $relflag $fflags >/dev/null 2>&1; then
            echo "  BUILD FAILED"; fail=1; continue
        fi
        export RUST_DRIVER_SO="$PWD/target/$profile/libdriver.so"
        # shellcheck disable=SC2086
        out=$(cargo test $relflag $fflags 2>&1)
        echo "$out" | grep -E '^test result|^error' | sed 's/^/  /'
        if echo "$out" | grep -qE '^test .* FAILED|^error'; then
            echo "  ---- FAILURES ----"
            echo "$out" | grep -E '^test .* FAILED|panicked at' | head -20 | sed 's/^/  /'
            fail=1
        fi
        unset RUST_DRIVER_SO
    done
done

# --- raw symbol diff ----------------------------------------------------------
echo "=============================================================="
echo "nm -D symbol diff (C vs Rust)"
echo "=============================================================="
syms() { nm -D --defined-only --format=posix "$1" 2>/dev/null \
    | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $1}' \
    | grep -vE '^(__|_init$|_fini$|_edata$|_end$|_IO_stdin_used$|_ITM_)' | sort -u; }
syms ../c_src/build/libdriver.so >"${TMPDIR:-/tmp}/c.syms"
syms target/release/libdriver.so >"${TMPDIR:-/tmp}/r.syms"
echo "C   exports: $(tr '\n' ' ' <"${TMPDIR:-/tmp}/c.syms")"
echo "RUST exports: $(tr '\n' ' ' <"${TMPDIR:-/tmp}/r.syms")"
missing=$(comm -23 "${TMPDIR:-/tmp}/c.syms" "${TMPDIR:-/tmp}/r.syms")
if [ -n "$missing" ]; then
    echo "MISSING FROM RUST: $missing"; fail=1
else
    echo "symbol diff: EMPTY (all C exports present in the Rust .so)"
fi

echo
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "SOME CONFIGURATIONS FAILED"; fi
exit "$fail"

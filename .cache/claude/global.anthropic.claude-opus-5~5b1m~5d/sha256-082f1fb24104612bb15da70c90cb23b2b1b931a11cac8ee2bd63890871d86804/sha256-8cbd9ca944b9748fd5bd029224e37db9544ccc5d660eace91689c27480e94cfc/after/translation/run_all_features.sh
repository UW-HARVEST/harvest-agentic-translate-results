#!/usr/bin/env bash
# Phase D driver: symbol parity + the full test suite under every build
# configuration (feature combination x cargo profile x C optimisation level).
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0

echo "###############################################################"
echo "# Feature combinations declared in Cargo.toml"
echo "###############################################################"
FEATURES=$(python3 - <<'PY'
import re, pathlib
s = pathlib.Path('Cargo.toml').read_text()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M | re.S)
if not m:
    print("(none)")
else:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            print(line.split('=')[0].strip())
PY
)
echo "$FEATURES"

# Build every C variant we compare against.
declare -A C_SOS
for cfg in "" "-DCMAKE_BUILD_TYPE=Release" "-DCMAKE_BUILD_TYPE=RelWithDebInfo" "-DCMAKE_BUILD_TYPE=Debug"; do
    name="${cfg#-DCMAKE_BUILD_TYPE=}"; name="${name:-default}"
    bdir="${TMPDIR:-/tmp}/cbuild_$name"
    rm -rf "$bdir"
    cmake -S "$ROOT/c_src" -B "$bdir" -DCMAKE_POSITION_INDEPENDENT_CODE=ON $cfg >/dev/null 2>&1
    cmake --build "$bdir" >/dev/null 2>&1
    so=$(ls "$bdir"/*.so 2>/dev/null | head -1)
    if [ -z "$so" ]; then echo "FAILED to build C variant $name"; FAIL=1; continue; fi
    C_SOS[$name]="$so"
    echo "C variant $name -> $so"
done

echo
echo "###############################################################"
echo "# Symbol parity (nm -D)"
echo "###############################################################"
cargo build --offline --release >/dev/null 2>&1
cargo build --offline >/dev/null 2>&1
for name in "${!C_SOS[@]}"; do
    for prof in release debug; do
        RSO="target/$prof/libima_parse_lib.so"
        cdefs=$(nm -D --defined-only "${C_SOS[$name]}" | awk '{print $3}' | sort -u)
        rdefs=$(nm -D --defined-only "$RSO" | awk '{print $3}' | sort -u)
        missing=$(comm -23 <(echo "$cdefs") <(echo "$rdefs"))
        if [ -n "$missing" ]; then
            echo "MISSING in $RSO (vs C/$name): $missing"; FAIL=1
        else
            echo "OK  C/$name  ->  $RSO : 0 missing symbols  ($(echo "$cdefs" | wc -l) exported)"
        fi
        # Truly unresolved (non-libc) symbols, as reported by the loader itself.
        unres=$(ldd -r "$RSO" 2>&1 | grep "undefined symbol" || true)
        if [ -n "$unres" ]; then
            echo "  UNRESOLVED in $RSO:"; echo "$unres" | sed 's/^/    /'; FAIL=1
        else
            echo "    ldd -r: 0 unresolved symbols in $RSO"
        fi
    done
done

echo
echo "###############################################################"
echo "# Differential test matrix"
echo "###############################################################"
run() { # $1=label  rest=cargo args
    local label="$1"; shift
    echo "--- $label"
    local out
    out=$(timeout 600 cargo test --offline "$@" 2>&1)
    echo "$out" | grep -E "^test result:" | sed 's/^/    /'
    if echo "$out" | grep -q "test result: FAILED"; then
        echo "    *** FAILED: $label"
        echo "$out" | grep -E "^---- |^test .* FAILED|panicked at" | sed 's/^/      /'
        FAIL=1
    fi
}

for cname in "${!C_SOS[@]}"; do
    export C_SO="${C_SOS[$cname]}"
    for combo in "default" "--no-default-features" "--all-features"; do
        args=()
        [ "$combo" != "default" ] && args+=("$combo")
        # release profile Rust .so
        RUST_SO="$PWD/target/release/libima_parse_lib.so" \
            run "C=$cname  features=$combo  rust=release" --release "${args[@]}"
        # debug profile Rust .so, exercised by the release test binaries
        RUST_SO="$PWD/target/debug/libima_parse_lib.so" \
            run "C=$cname  features=$combo  rust=debug" --release "${args[@]}"
    done
done

echo
if [ "$FAIL" = 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "SOME CONFIGURATIONS FAILED"; fi
exit $FAIL

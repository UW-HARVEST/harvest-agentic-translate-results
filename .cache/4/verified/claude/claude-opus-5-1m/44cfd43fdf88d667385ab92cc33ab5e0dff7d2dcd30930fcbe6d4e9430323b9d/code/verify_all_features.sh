#!/usr/bin/env bash
# Phase D driver: enumerate EVERY valid feature combination from Cargo.toml and
# run `cargo check` + the full differential test suite for each one.
#
# The feature list is extracted mechanically from Cargo.toml (no hard-coded
# names), so adding a feature automatically widens the matrix.
#
# usage: ./verify_all_features.sh [--release]
set -uo pipefail

cd "$(dirname "$0")"

PROFILE_ARGS=()
PROFILE_LABEL="dev"
if [[ "${1:-}" == "--release" ]]; then
    PROFILE_ARGS=(--release)
    PROFILE_LABEL="release"
fi

OFFLINE=()
if [[ -n "${CARGO_OFFLINE:-1}" ]]; then OFFLINE=(--offline); fi

# ---------------------------------------------------------------------------
# 1. Build the C reference shared library
# ---------------------------------------------------------------------------
echo "== building the C reference .so =="
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=c_src/build/libtranslated_rust.so
test -f "$C_SO" || { echo "missing $C_SO"; exit 1; }

# ---------------------------------------------------------------------------
# 2. Enumerate the feature power set from Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t COMBOS < <(python3 - <<'PY'
import itertools, re, sys

src = open("Cargo.toml").read()

# Grab the [features] table (if any) and collect its keys, skipping the
# implicit-dependency ("dep:") style values.
feats = []
m = re.search(r"^\[features\]\s*$(.*?)(?=^\[|\Z)", src, re.M | re.S)
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=', 1)[0].strip().strip('"')
        if name and name != "default":
            feats.append(name)

# Every subset of the optional features, always with --no-default-features so
# the combination is exact; plus the plain default build and --all-features.
print("__DEFAULT__")
for k in range(len(feats) + 1):
    for combo in itertools.combinations(sorted(feats), k):
        print("__NODEFAULT__" + ",".join(combo))
if feats:
    print("__ALL__")
PY
)

echo
echo "== ${#COMBOS[@]} feature combination(s) to verify (profile: $PROFILE_LABEL) =="
for c in "${COMBOS[@]}"; do echo "   - $c"; done
echo

fail=0
for combo in "${COMBOS[@]}"; do
    case "$combo" in
        __DEFAULT__)     ARGS=() ;                                   LABEL="(default features)" ;;
        __ALL__)         ARGS=(--all-features) ;                     LABEL="--all-features" ;;
        __NODEFAULT__)   ARGS=(--no-default-features) ;              LABEL="--no-default-features" ;;
        __NODEFAULT__*)  f="${combo#__NODEFAULT__}"
                         ARGS=(--no-default-features --features "$f"); LABEL="--no-default-features --features $f" ;;
        *)               ARGS=(--features "$combo") ;                LABEL="--features $combo" ;;
    esac

    echo "-------------------------------------------------------------------"
    echo ">> cargo check $LABEL"
    if ! timeout 600 cargo check "${OFFLINE[@]}" "${PROFILE_ARGS[@]}" "${ARGS[@]}" \
            --all-targets 2>&1 | tail -n 5; then
        echo "!! CHECK FAILED for $LABEL"; fail=1; continue
    fi

    # The integration tests dlopen `libreverse_collide_lib.so`, and `cargo test`
    # alone does NOT build a `cdylib`-only lib target -- it must be built
    # explicitly, with the very same profile and feature set.
    echo ">> cargo build $LABEL (produces the cdylib the tests dlopen)"
    if ! timeout 600 cargo build "${OFFLINE[@]}" "${PROFILE_ARGS[@]}" "${ARGS[@]}" 2>&1 | tail -n 3; then
        echo "!! BUILD FAILED for $LABEL"; fail=1; continue
    fi

    echo ">> cargo test $LABEL"
    out=$(timeout 600 cargo test "${OFFLINE[@]}" "${PROFILE_ARGS[@]}" "${ARGS[@]}" 2>&1)
    echo "$out" | grep -E "^test result|^error|FAILED|panicked" || true
    if echo "$out" | grep -qE "FAILED|^error|panicked"; then
        echo "!! TESTS FAILED for $LABEL"
        echo "$out" | tail -n 40
        fail=1
        continue
    fi

    # Symbol parity for this configuration.
    if [[ "$PROFILE_LABEL" == "release" ]]; then
        RUST_SO=target/release/libreverse_collide_lib.so
    else
        RUST_SO=target/debug/libreverse_collide_lib.so
    fi
    if [[ ! -f "$RUST_SO" ]]; then echo "!! no Rust .so produced at $RUST_SO"; fail=1; continue; fi
    CS="${TMPDIR:-/tmp}/c.$$"; RS="${TMPDIR:-/tmp}/r.$$"
    nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort -u > "$CS"
    nm -D --defined-only "$RUST_SO"| awk '{print $NF}' | sort -u > "$RS"
    miss=$(comm -23 "$CS" "$RS")
    extra=$(comm -13 "$CS" "$RS")
    if [[ -n "$miss" || -n "$extra" ]]; then
        echo "!! SYMBOL PARITY FAILED for $LABEL"
        [[ -n "$miss"  ]] && echo "   missing from Rust: $miss"
        [[ -n "$extra" ]] && echo "   extra in Rust:     $extra"
        fail=1
    else
        echo ">> symbol parity OK ($(wc -l < "$CS") symbols, 0 missing, 0 extra)"
    fi
    rm -f "$CS" "$RS"
done

echo "==================================================================="
if [[ $fail -eq 0 ]]; then
    echo "ALL FEATURE COMBINATIONS PASSED (profile: $PROFILE_LABEL)"
else
    echo "FAILURES DETECTED (profile: $PROFILE_LABEL)"
fi
exit $fail

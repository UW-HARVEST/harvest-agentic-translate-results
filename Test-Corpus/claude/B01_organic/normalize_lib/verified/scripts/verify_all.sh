#!/usr/bin/env bash
# Full verification matrix:
#   * build the C shared library (its single cmake configuration)
#   * enumerate EVERY feature combination declared in Cargo.toml (powerset of
#     [features] minus `default`)
#   * for every combination x {dev, release} profile: cargo check, cargo build
#     (produces the cdylib the tests dlopen) and cargo test (Phases B, C, D)
#
# Usage: scripts/verify_all.sh [extra cargo args...]
set -u -o pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT=$(pwd)
LOGDIR=${TMPDIR:-/tmp}/verify_all
mkdir -p "$LOGDIR"
EXTRA=("--offline" "$@")
rc=0

printf '== building the C shared library ==\n'
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) > "$LOGDIR/cmake.log" 2>&1
if [ $? -ne 0 ]; then
    printf 'FAIL: C build\n'; tail -n 20 "$LOGDIR/cmake.log"; exit 1
fi
ls -l c_src/build/*.so

# ---- enumerate feature combinations -------------------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}' Cargo.toml
)
n=${#FEATURES[@]}
printf '== %d optional feature(s) declared: %s ==\n' "$n" "${FEATURES[*]:-<none>}"

COMBOS=()
total=$((1 << n))
for ((mask = 0; mask < total; mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
        if (( (mask >> b) & 1 )); then
            combo="${combo:+$combo,}${FEATURES[$b]}"
        fi
    done
    COMBOS+=("$combo")
done
printf '== %d feature combination(s) to verify ==\n' "${#COMBOS[@]}"

for combo in "${COMBOS[@]}"; do
    label=${combo:-<none>}
    for profile in dev release; do
        tag=$(printf '%s_%s' "${combo:-none}" "$profile" | tr ',' '+')
        relflag=()
        [ "$profile" = release ] && relflag=(--release)

        printf '\n----- features=%s profile=%s -----\n' "$label" "$profile"

        if ! timeout 600 cargo check "${EXTRA[@]}" --no-default-features \
                --features "$combo" --all-targets "${relflag[@]}" \
                > "$LOGDIR/check_$tag.log" 2>&1; then
            printf 'FAIL cargo check (features=%s, %s)\n' "$label" "$profile"
            tail -n 30 "$LOGDIR/check_$tag.log"; rc=1; continue
        fi
        printf 'cargo check   OK\n'

        if ! timeout 600 cargo build "${EXTRA[@]}" --no-default-features \
                --features "$combo" "${relflag[@]}" \
                > "$LOGDIR/build_$tag.log" 2>&1; then
            printf 'FAIL cargo build (features=%s, %s)\n' "$label" "$profile"
            tail -n 30 "$LOGDIR/build_$tag.log"; rc=1; continue
        fi
        printf 'cargo build   OK -> %s\n' "$(ls target/*/libnormalize_lib.so 2>/dev/null | tr '\n' ' ')"

        if ! timeout 600 cargo test "${EXTRA[@]}" --no-default-features \
                --features "$combo" "${relflag[@]}" -- --test-threads=4 \
                > "$LOGDIR/test_$tag.log" 2>&1; then
            printf 'FAIL cargo test (features=%s, %s)\n' "$label" "$profile"
            grep -E "^(test |error|failures:)" -A 4 "$LOGDIR/test_$tag.log" | tail -n 60
            rc=1; continue
        fi
        grep -hE "^test result:" "$LOGDIR/test_$tag.log" | sed 's/^/cargo test    /'
    done
done

printf '\n== symbol parity (nm -D diff) ==\n'
CSO=c_src/build/libtranslated_rust.so
for RSO in target/debug/libnormalize_lib.so target/release/libnormalize_lib.so; do
    [ -f "$RSO" ] || continue
    cdef=$(nm -D --defined-only "$CSO" | awk '{print $NF}' | sort -u)
    rdef=$(nm -D --defined-only "$RSO" | awk '{print $NF}' | sort -u)
    missing=$(comm -23 <(printf '%s\n' "$cdef") <(printf '%s\n' "$rdef"))
    printf '%s: C exports %d symbol(s), Rust exports %d symbol(s)\n' \
        "$RSO" "$(printf '%s\n' "$cdef" | grep -c .)" "$(printf '%s\n' "$rdef" | grep -c .)"
    if [ -n "$missing" ]; then
        printf 'FAIL: missing from %s:\n%s\n' "$RSO" "$missing"; rc=1
    else
        printf 'symbol diff EMPTY (0 missing)\n'
    fi
done

printf '\n== RESULT: %s ==\n' "$([ $rc -eq 0 ] && printf 'ALL CHECKS PASSED' || printf 'FAILURES PRESENT')"
exit $rc

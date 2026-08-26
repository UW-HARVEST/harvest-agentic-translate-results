#!/usr/bin/env bash
# Full verification driver: builds the C reference .so, enumerates every valid
# Cargo feature combination, and for each one runs `cargo check`, builds the
# Rust cdylib, checks symbol parity and runs the Phase B + Phase C differential
# suites.  Both the debug and the release profile are covered (the release
# profile differs: panic = "abort", optimisations on).
set -u
cd "$(dirname "$0")"

CARGO_FLAGS="--offline"

echo "############ building the C reference shared library ############"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build failed"; exit 1; }
ls -l c_src/build/libString_Slice.so

# ---------------------------------------------------------------------------
# Enumerate the feature power set straight out of Cargo.toml.
# ---------------------------------------------------------------------------
feats=$(awk '
    /^\[features\]/ {inside=1; next}
    /^\[/ {inside=0}
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
        split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
        if (a[1] != "default") print a[1]
    }' Cargo.toml)
n=0
for f in $feats; do n=$((n+1)); done
echo
echo "############ feature enumeration ############"
echo "non-default features declared: ${feats:-<none>} (count=$n)"

combos=()
total=$((1 << n))
for ((mask = 0; mask < total; mask++)); do
    sel=""; i=0
    for f in $feats; do
        if (( (mask >> i) & 1 )); then sel="${sel:+$sel,}$f"; fi
        i=$((i+1))
    done
    combos+=("$sel")
done
echo "feature combinations to verify: ${#combos[@]}"

rc=0
for profile in debug release; do
    relflag=""
    [ "$profile" = release ] && relflag="--release"
    for combo in "${combos[@]}"; do
        label="profile=$profile features=[${combo:-<empty>}]"
        echo
        echo "################################################################"
        echo "# $label"
        echo "################################################################"

        # shellcheck disable=SC2086
        if ! cargo check $CARGO_FLAGS --no-default-features ${combo:+--features "$combo"} \
             --all-targets $relflag; then
            echo "cargo check FAILED for $label"; rc=1; continue
        fi
        # shellcheck disable=SC2086
        if ! cargo build $CARGO_FLAGS --no-default-features ${combo:+--features "$combo"} $relflag; then
            echo "cargo build FAILED for $label"; rc=1; continue
        fi
        if ! PROFILE="$profile" ./check_symbols.sh; then
            echo "symbol parity FAILED for $label"; rc=1
        fi
        # shellcheck disable=SC2086
        if ! cargo test $CARGO_FLAGS --no-default-features ${combo:+--features "$combo"} $relflag; then
            echo "differential tests FAILED for $label"; rc=1
        fi
    done
done

# The default feature set must also be exercised as a plain `cargo test`.
echo
echo "################################################################"
echo "# default feature set (and --all-features), debug profile"
echo "################################################################"
cargo test $CARGO_FLAGS || rc=1
cargo test $CARGO_FLAGS --all-features || rc=1

echo
if [ "$rc" -eq 0 ]; then
    echo "############ ALL CONFIGURATIONS VERIFIED ############"
else
    echo "############ VERIFICATION FAILED ############"
fi
exit "$rc"

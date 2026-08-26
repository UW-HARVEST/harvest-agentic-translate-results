#!/usr/bin/env bash
# Full verification sweep: every feature combination x every profile.
#
# Cargo.toml declares no [features] (and c_src/CMakeLists.txt declares no build
# options), so the complete set of valid feature combinations is:
#   1. default            -> `cargo test`
#   2. no default features -> `cargo test --no-default-features`
# (both are identical here, and both are checked so the claim is verified rather
# than assumed).
set -uo pipefail
cd "$(dirname "$0")"

# Prefer offline (the crates are cached); fall back to a normal online build.
if cargo metadata --offline --format-version 1 >/dev/null 2>&1; then
    CARGO_FLAGS="--offline"
else
    CARGO_FLAGS=""
fi
rc=0

echo "############ feature enumeration ############"
awk '/^\[features\]/{f=1;next} /^\[/{f=0} f&&NF' Cargo.toml || true
echo "(no [features] section => combinations: {default}, {--no-default-features})"

./build_c.sh

for featflag in "" "--no-default-features" "--all-features"; do
    echo
    echo "############ cargo check ${featflag:-<default>} ############"
    cargo check $CARGO_FLAGS $featflag --all-targets 2>&1 | tail -3 || rc=1
done

for profile in dev release; do
    if [ "$profile" = "release" ]; then
        prof_flag="--release"; prof_dir=release
    else
        prof_flag=""; prof_dir=debug
    fi
    for featflag in "" "--no-default-features"; do
        echo
        echo "############ profile=$profile features=${featflag:-<default>} ############"
        cargo build $CARGO_FLAGS $prof_flag $featflag 2>&1 | tail -2 || rc=1
        ./check_symbols.sh "$prof_dir" || rc=1
        log="${TMPDIR:-/tmp}/verify-$profile-${featflag:-default}.log"
        if timeout 900 cargo test $CARGO_FLAGS $prof_flag $featflag -- --test-threads=1 \
                > "$log" 2>&1; then
            grep -E "^(running|test result)" "$log"
        else
            echo "TESTS FAILED (see $log)"
            grep -E "FAILED|panicked|error" "$log" | head -20
            rc=1
        fi
    done
done

echo
if [ $rc -eq 0 ]; then
    echo "ALL CONFIGURATIONS PASSED"
else
    echo "FAILURES PRESENT (rc=$rc)"
fi
exit $rc

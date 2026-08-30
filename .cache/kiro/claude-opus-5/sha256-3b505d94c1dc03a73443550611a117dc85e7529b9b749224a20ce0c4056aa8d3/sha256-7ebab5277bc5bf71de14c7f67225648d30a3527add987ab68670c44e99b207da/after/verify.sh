#!/usr/bin/env bash
# Runs the differential test suite for every declared feature combination, and
# for every combination of C / Rust optimisation levels.
#
#   ./verify.sh          from the repository root (the directory holding c_src/)
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$root"

fail=0

# --- Step 1: enumerate feature combinations ---------------------------------
# Read the [features] table out of Cargo.toml. `default` is expanded by cargo
# itself, so only the explicitly declared non-default features are combined.
mapfile -t features < <(python3 - <<'PY'
import itertools, re, sys
text = open("translation/Cargo.toml").read()
match = re.search(r"^\[features\]\s*$(.*?)(?=^\[|\Z)", text, re.S | re.M)
names = []
if match:
    for line in match.group(1).splitlines():
        line = line.split("#", 1)[0].strip()
        if not line or "=" not in line:
            continue
        name = line.split("=", 1)[0].strip().strip('"')
        if name != "default":
            names.append(name)
for size in range(len(names) + 1):
    for combo in itertools.combinations(names, size):
        print(",".join(combo))
PY
)

echo "feature combinations: ${#features[@]}"
for combo in "${features[@]}"; do
    echo "  - '${combo:-<none>}'"
done

# --- Step 2: cargo check every combination ----------------------------------
for combo in "${features[@]}"; do
    echo "== cargo check --no-default-features --features '${combo}'"
    if ! timeout 600 cargo check --manifest-path translation/Cargo.toml \
        --no-default-features --features "$combo" >/tmp/check.log 2>&1; then
        echo "FAILED: cargo check '${combo}'"
        tail -30 /tmp/check.log
        fail=1
    fi
done
# The default feature set is what an ordinary consumer gets.
echo "== cargo check (default features)"
timeout 600 cargo check --manifest-path translation/Cargo.toml >/tmp/check.log 2>&1 \
    || { echo "FAILED: cargo check default"; tail -30 /tmp/check.log; fail=1; }

# --- Step 3: build the C shared object at several optimisation levels -------
# `(int)(100.0 / 0.0)` is undefined behaviour in C, so the emitted conversion is
# checked at every optimisation level rather than only at the CMake default.
declare -A c_libs
for opt in default O0 O2 O3 Ofast; do
    # Built outside c_src so that subtree is left untouched.
    build="build-c/$opt"
    flags=""
    [ "$opt" != "default" ] && flags="-$opt"
    rm -rf "$build"
    if ! cmake -S c_src -B "$build" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DCMAKE_C_FLAGS="$flags" >/tmp/cmake.log 2>&1; then
        echo "FAILED: cmake configure $opt"; tail -20 /tmp/cmake.log; fail=1; continue
    fi
    if ! timeout 600 cmake --build "$build" >/tmp/cbuild.log 2>&1; then
        echo "FAILED: cmake build $opt"; tail -20 /tmp/cbuild.log; fail=1; continue
    fi
    c_libs[$opt]="$root/$build/libdriver.so"
done

# --- Step 4: build the Rust shared object per profile and feature combo -----
build_rust() {
    local profile="$1" combo="$2" target_dir="$3"
    local args=(build --lib --manifest-path translation/Cargo.toml --no-default-features)
    [ -n "$combo" ] && args+=(--features "$combo")
    [ "$profile" = release ] && args+=(--release)
    CARGO_TARGET_DIR="$target_dir" timeout 600 cargo "${args[@]}" >/tmp/rbuild.log 2>&1 \
        || { echo "FAILED: cargo build $profile '$combo'"; tail -20 /tmp/rbuild.log; return 1; }
    echo "$target_dir/$profile/libdriver.so"
}

# --- Step 5: symbol parity + differential tests -----------------------------
for combo in "${features[@]}"; do
    for profile in debug release; do
        target_dir="$root/translation/target/verify-$profile-${combo//,/+}"
        rust_so="$(build_rust "$profile" "$combo" "$target_dir")" || { fail=1; continue; }

        for opt in "${!c_libs[@]}"; do
            c_so="${c_libs[$opt]}"

            # Every symbol the C .so exports must be exported by the Rust .so.
            missing="$(comm -23 \
                <(nm -D --defined-only "$c_so" | awk '$2 ~ /^[TDBWRV]$/ {print $3}' | sort -u) \
                <(nm -D --defined-only "$rust_so" | awk '$2 ~ /^[TDBWRV]$/ {print $3}' | sort -u))"
            if [ -n "$missing" ]; then
                echo "FAILED: symbols missing from Rust .so (features='$combo' profile=$profile C=$opt):"
                echo "$missing" | sed 's/^/    /'
                fail=1
            fi

            echo "== test features='${combo:-<none>}' rust=$profile c=$opt"
            args=(test --manifest-path translation/Cargo.toml --no-default-features)
            [ -n "$combo" ] && args+=(--features "$combo")
            if ! C_DRIVER_SO="$c_so" RUST_DRIVER_SO="$rust_so" \
                timeout 600 cargo "${args[@]}" >/tmp/test.log 2>&1; then
                echo "FAILED: features='$combo' rust=$profile c=$opt"
                grep -E "^(test |failures:|thread |  left| right|assertion)" /tmp/test.log | head -40
                fail=1
            fi
        done
    done
done

if [ "$fail" -eq 0 ]; then
    echo "ALL CONFIGURATIONS PASSED"
else
    echo "SOME CONFIGURATIONS FAILED"
fi
exit "$fail"

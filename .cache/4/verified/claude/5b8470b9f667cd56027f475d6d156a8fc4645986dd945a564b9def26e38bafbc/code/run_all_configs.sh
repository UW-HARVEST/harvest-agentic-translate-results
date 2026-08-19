#!/usr/bin/env bash
# Runs the whole differential verification suite under EVERY build
# configuration:
#
#   1. every valid Cargo feature combination (enumerated from Cargo.toml —
#      the crate declares no [features], so the power set is the single empty
#      combo, but the enumeration is mechanical so it stays correct if features
#      are ever added),
#   2. the dev and the release profile of the Rust cdylib (release adds
#      opt-level=3 and panic="abort", a genuinely different code path),
#   3. the C reference at several optimisation levels, because glibc's
#      <ctype.h> switches tolower/toupper between an out-of-line call and an
#      inline table lookup depending on __OPTIMIZE__.  c_src/ is never modified:
#      the extra builds are compiled straight into target/.
#
# Every configuration also gets its `nm -D` exported-symbol set diffed against
# the C reference.

set -uo pipefail
cd "$(dirname "$0")"

CARGO_FLAGS=(--offline)
FAILURES=()
run_step() {
    local name="$1"
    shift
    echo
    echo "=============================================================="
    echo ">>> $name"
    echo "    \$ $*"
    echo "=============================================================="
    if timeout 600 "$@"; then
        echo "--- PASS: $name"
    else
        echo "--- FAIL: $name"
        FAILURES+=("$name")
    fi
}

# ---------------------------------------------------------------------------
# 0. The C reference build (the ground truth), exactly as documented
# ---------------------------------------------------------------------------
if [ ! -f c_src/build/libdriver.so ]; then
    ( mkdir -p c_src/build && cd c_src/build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build failed"; exit 1; }
fi
C_REF=c_src/build/libdriver.so
echo "C reference: $C_REF"

# ---------------------------------------------------------------------------
# 1. Enumerate every valid feature combination from Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
    awk '
        /^\[features\]/ { inside = 1; next }
        /^\[/           { inside = 0 }
        inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
            if (a[1] != "default") print a[1];
        }
    ' Cargo.toml
)
echo "declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

COMBOS=("")
for (( mask = 1; mask < (1 << ${#FEATURES[@]}); mask++ )); do
    combo=""
    for (( bit = 0; bit < ${#FEATURES[@]}; bit++ )); do
        if (( mask & (1 << bit) )); then
            combo="${combo:+$combo,}${FEATURES[$bit]}"
        fi
    done
    COMBOS+=("$combo")
done
echo "feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  - '${c:-<no features>}'"; done

symbol_diff() {
    local label="$1" rust_so="$2"
    local c_syms rust_syms
    c_syms=$(nm -D --defined-only "$C_REF" | awk '{print $NF}' | sort -u)
    rust_syms=$(nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort -u)
    if diff <(echo "$c_syms") <(echo "$rust_syms") > target/symdiff.$$ 2>&1; then
        echo "--- PASS: symbol parity ($label): $(echo "$c_syms" | tr '\n' ' ')"
    else
        echo "--- FAIL: symbol parity ($label):"
        cat target/symdiff.$$
        FAILURES+=("symbol parity ($label)")
    fi
    rm -f target/symdiff.$$
}

# ---------------------------------------------------------------------------
# 2. cargo check + cargo test for every feature combination (dev profile)
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
    label="features='${combo:-<none>}'"
    run_step "cargo check ($label, all targets)" \
        cargo check "${CARGO_FLAGS[@]}" --no-default-features --features "$combo" --all-targets
    run_step "cargo build cdylib ($label)" \
        cargo build "${CARGO_FLAGS[@]}" --no-default-features --features "$combo"
    symbol_diff "$label, dev" target/debug/libdriver.so
    run_step "cargo test ($label, dev profile)" \
        cargo test "${CARGO_FLAGS[@]}" --no-default-features --features "$combo"
done

# ---------------------------------------------------------------------------
# 3. Release profile (opt-level 3 + panic = "abort")
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
    label="features='${combo:-<none>}'"
    run_step "cargo build --release ($label)" \
        cargo build "${CARGO_FLAGS[@]}" --release --no-default-features --features "$combo"
    symbol_diff "$label, release" target/release/libdriver.so
    RUST_DRIVER_SO="$PWD/target/release/libdriver.so" \
        run_step "cargo test against the RELEASE cdylib ($label)" \
            cargo test "${CARGO_FLAGS[@]}" --no-default-features --features "$combo"
done

# ---------------------------------------------------------------------------
# 4. Further Rust codegen configurations
#
# The one real divergence this suite found was visible only under optimisation
# (LLVM exploiting the `signext` parameter attribute), so the optimiser settings
# are themselves a configuration axis worth sweeping.
# ---------------------------------------------------------------------------
mkdir -p target/rust-variants
rust_variant() {
    local label="$1" built="$2" dest="target/rust-variants/$3"
    shift 3
    if env "$@" timeout 600 cargo build "${CARGO_FLAGS[@]}" ${BUILD_EXTRA:-} > /dev/null 2>&1; then
        cp "$built" "$dest"
        symbol_diff "$label" "$dest"
        RUST_DRIVER_SO="$PWD/$dest" run_step "cargo test against the Rust build: $label" \
            cargo test "${CARGO_FLAGS[@]}" --no-default-features
    else
        echo "--- FAIL: could not build Rust variant $label"
        FAILURES+=("build Rust variant $label")
    fi
}

BUILD_EXTRA="--release" rust_variant "release + fat LTO" \
    target/release/libdriver.so libdriver-release-lto.so \
    CARGO_PROFILE_RELEASE_LTO=fat
BUILD_EXTRA="--release" rust_variant "release + panic=unwind" \
    target/release/libdriver.so libdriver-release-unwind.so \
    CARGO_PROFILE_RELEASE_PANIC=unwind
BUILD_EXTRA="" rust_variant "dev + opt-level=2" \
    target/debug/libdriver.so libdriver-dev-opt2.so \
    CARGO_PROFILE_DEV_OPT_LEVEL=2
BUILD_EXTRA="--release" rust_variant "release + opt-level=s" \
    target/release/libdriver.so libdriver-release-opt-s.so \
    CARGO_PROFILE_RELEASE_OPT_LEVEL=s

# Restore the canonical dev/release artifacts the other steps rely on.
cargo build "${CARGO_FLAGS[@]}" > /dev/null 2>&1
cargo build "${CARGO_FLAGS[@]}" --release > /dev/null 2>&1

# ---------------------------------------------------------------------------
# 5. The C reference at other optimisation levels (c_src/ is left untouched)
# ---------------------------------------------------------------------------
mkdir -p target/c-variants
for opt in "-O0" "-O1" "-O2" "-O3" "-Os" "-O2 -D_FORTIFY_SOURCE=2" "-O2 -fno-builtin"; do
    tag=$(echo "$opt" | tr -d ' =' )
    so="$PWD/target/c-variants/libdriver$tag.so"
    if gcc $opt -fPIC -shared -Ic_src/include c_src/src/driver.c -o "$so" 2>/dev/null; then
        echo
        echo ">>> C reference rebuilt with '$opt' -> $so"
        echo "    imports: $(nm -D --undefined-only "$so" | awk '{print $NF}' | tr '\n' ' ')"
        C_DRIVER_SO="$so" run_step "cargo test against the C build at '$opt'" \
            cargo test "${CARGO_FLAGS[@]}" --no-default-features
    else
        echo "skip: gcc $opt build failed"
    fi
done

# ---------------------------------------------------------------------------
# 6. Summary
# ---------------------------------------------------------------------------
echo
echo "=============================================================="
if [ ${#FAILURES[@]} -eq 0 ]; then
    echo "ALL CONFIGURATIONS PASSED"
    exit 0
else
    echo "FAILURES (${#FAILURES[@]}):"
    printf '  - %s\n' "${FAILURES[@]}"
    exit 1
fi

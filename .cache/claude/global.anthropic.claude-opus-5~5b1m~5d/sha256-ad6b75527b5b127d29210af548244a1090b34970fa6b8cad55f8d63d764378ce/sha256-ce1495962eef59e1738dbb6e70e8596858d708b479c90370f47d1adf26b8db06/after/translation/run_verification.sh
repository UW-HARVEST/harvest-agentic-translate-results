#!/usr/bin/env bash
#
# Full verification run: builds the reference C library and the Rust cdylib,
# then runs the differential suite against every configuration.
#
#   ./run_verification.sh              # normal run (~3 min)
#   PINFLATE_FULL_SWEEP=1 ./run_verification.sh
#                                      # + all 65536 two-byte inputs (~8 min)
#   ./run_verification.sh b_dyn        # only test ids containing "b_dyn"
#
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
filter="${1:-}"
fail=0

step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
bad()  { printf '\033[31mFAILED: %s\033[0m\n' "$*"; fail=1; }

# ---------------------------------------------------------------- C reference
step "building the C reference library"
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || bad "C build"
c_so="$(ls "$root"/c_src/build/lib*.so 2>/dev/null | head -1)"
[ -n "$c_so" ] || bad "no C .so produced"
echo "C library: $c_so"

# ---------------------------------------------------------------- Rust cdylib
# `cargo test` does not emit the cdylib, so build it explicitly, in both
# profiles: the dev profile is unoptimised, the release profile is optimised and
# has `panic = "abort"`, and the two are the only distinct configurations of this
# crate (Cargo.toml declares no [features]).
step "enumerating feature combinations"
feats="$(sed -n '/^\[features\]/,/^\[/p' "$here/Cargo.toml" | grep -E '^[a-zA-Z0-9_-]+ *=' | cut -d= -f1 | tr -d ' ')"
if [ -z "$feats" ]; then
    echo "Cargo.toml declares no [features] -> the only configurations are the"
    echo "dev and release profiles."
    combos=("")
else
    combos=("")
    for f in $feats; do combos+=("$f"); done
    combos+=("$(echo "$feats" | tr '\n' ',' | sed 's/,$//')")
fi
for c in "${combos[@]}"; do
    step "cargo check --features '${c}'"
    if [ -z "$c" ]; then
        ( cd "$here" && cargo check --offline --all-targets ) || bad "cargo check (default)"
        ( cd "$here" && cargo check --offline --no-default-features ) || bad "cargo check --no-default-features"
    else
        ( cd "$here" && cargo check --offline --no-default-features --features "$c" ) \
            || bad "cargo check --features $c"
    fi
done

step "building the Rust cdylib (dev + release)"
( cd "$here" && cargo build --offline )           || bad "cargo build (dev)"
( cd "$here" && cargo build --offline --release )  || bad "cargo build (release)"
( cd "$here" && cargo build --offline --tests )    || bad "cargo build --tests"

# ---------------------------------------------------------------- symbol parity
step "Phase D: symbol parity (nm -D)"
( cd "$here" && cargo test --offline --test symbols -- --nocapture ) || bad "symbol parity"

for so in debug release; do
    printf '\n%-28s %s\n' "nm -D diff ($so):" "$(
        diff <(nm -D --defined-only "$c_so" | awk '{print $NF}' | sort) \
             <(nm -D --defined-only "$here/target/$so/libpinflate_lib.so" \
                 | awk '{print $NF}' | sort) >/dev/null && echo EMPTY || echo NOT-EMPTY)"
done

# ---------------------------------------------------------- Phases B, C, D
for so in debug release; do
    step "Phases B + C against target/$so/libpinflate_lib.so"
    ( cd "$here" \
      && PINFLATE_RUST_SO="$here/target/$so/libpinflate_lib.so" \
         cargo test --offline --test differential -- $filter ) \
        || bad "differential suite ($so cdylib)"
done

step "summary"
if [ "$fail" -eq 0 ]; then
    echo "ALL PHASES PASSED"
else
    echo "THERE WERE FAILURES"
fi
exit "$fail"

#!/usr/bin/env bash
# Phase D driver: run the whole differential suite for every feature
# combination declared in Cargo.toml, against both the debug and the release
# cdylib, and diff the exported symbols of the C and Rust shared objects.
#
# Usage: tests/run_all_features.sh        (from the `translation/` directory)
set -uo pipefail

cd "$(dirname "$0")/.."          # translation/
CRATE_DIR="$PWD"
C_DIR="$CRATE_DIR/../c_src"
C_SO="$C_DIR/build/libdriver.so"
rc=0

hdr() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------- C library
hdr "build C shared library"
( mkdir -p "$C_DIR/build" && cd "$C_DIR/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
test -f "$C_SO" || { echo "missing $C_SO"; exit 1; }
echo "ok: $C_SO"

# --------------------------------------------------- feature set enumeration
# Every name under [features] in Cargo.toml, excluding "default".
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
         split($0,a,"="); gsub(/[[:space:]]/,"",a[1]); if (a[1]!="default") print a[1] }' Cargo.toml
)
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "Cargo.toml declares no [features]; the default build is the only configuration"
  COMBOS=("--")                     # placeholder: default features only
else
  COMBOS=("--")
  # all non-empty subsets of FEATURES, plus --no-default-features
  n=${#FEATURES[@]}
  for ((mask=0; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      (( mask & (1<<i) )) && combo="${combo:+$combo,}${FEATURES[i]}"
    done
    COMBOS+=("--no-default-features --features=$combo")
  done
fi

# ------------------------------------------------------------------ test runs
for combo in "${COMBOS[@]}"; do
  flags=""
  [ "$combo" != "--" ] && flags="$combo"
  for profile in debug release; do
    pflag=""
    [ "$profile" = release ] && pflag="--release"
    hdr "features '${flags:-<default>}' / $profile"

    # `cargo test` does not build the cdylib (integration tests do not link
    # it), so build it explicitly first.
    if ! timeout 600 cargo build $pflag $flags 2>&1 | tail -3; then
      echo "BUILD FAILED"; rc=1; continue
    fi
    export DRIVER_C_SO="$C_SO"
    export DRIVER_RUST_SO="$CRATE_DIR/target/$profile/libdriver.so"

    # symbol parity for this configuration
    if diff <(nm -D --defined-only "$C_SO"        | awk '{print $NF}' | sort) \
            <(nm -D --defined-only "$DRIVER_RUST_SO" | awk '{print $NF}' | sort) ; then
      echo "symbol parity ... ok"
    else
      echo "symbol parity ... FAILED (< C only, > Rust only)"; rc=1
    fi
    # no unresolved non-libc imports
    if ldd -r "$DRIVER_RUST_SO" 2>&1 | grep -q 'undefined symbol'; then
      echo "unresolved imports ... FAILED"
      ldd -r "$DRIVER_RUST_SO" 2>&1 | grep 'undefined symbol'
      rc=1
    else
      echo "unresolved imports ... ok"
    fi

    # tests always exercise the .so selected above via DRIVER_RUST_SO; the test
    # binaries themselves are built in debug to keep the run fast.
    if ! timeout 600 cargo test $flags --test differential 2>&1 | tail -4; then
      rc=1
    fi
    if ! timeout 600 cargo test $flags --test robustness 2>&1 | tail -20; then
      rc=1
    fi
    unset DRIVER_C_SO DRIVER_RUST_SO
  done
done

hdr "summary"
if [ "$rc" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$rc"

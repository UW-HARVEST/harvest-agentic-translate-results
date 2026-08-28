#!/usr/bin/env bash
# Full differential verification driver.
#
# Always REBUILDS both libraries before testing. This matters: `cargo test
# --test <name>` does not rebuild a `crate-type = ["cdylib"]` library, because
# the integration tests depend on it only through `dlopen`. Skipping the explicit
# build silently tests a stale `.so`. The harness also asserts freshness, but the
# build here is the primary defence.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT" || exit 1
FAIL=0

hdr() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }

# ---------------------------------------------------------------- C library ---
hdr "Building C shared library"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | head -1)
echo "C  .so: $C_SO"

# ------------------------------------------------------- feature enumeration ---
# Extract feature names from Cargo.toml's [features] table (excluding `default`).
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0,a,"="); gsub(/[[:space:]]/,"",a[1]);
    if (a[1] != "default") print a[1]
  }' translation/Cargo.toml)

# Build the list of feature combinations to test.
COMBOS=()
if [[ -z "$FEATURES" ]]; then
  echo "No [features] table in Cargo.toml -> single configuration."
  COMBOS+=("--no-default-features")   # identical to default when no features exist
  COMBOS+=("")                        # default build
else
  COMBOS+=("" "--no-default-features")
  # every non-empty subset of the feature set
  feat_arr=($FEATURES)
  n=${#feat_arr[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    sel=()
    for ((i=0; i<n; i++)); do (( mask & (1<<i) )) && sel+=("${feat_arr[$i]}"); done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
fi

# ------------------------------------------------------------------- matrix ---
cd translation || exit 1
for PROFILE in dev release; do
  PFLAG=""; [[ $PROFILE == release ]] && PFLAG="--release"
  for COMBO in "${COMBOS[@]}"; do
    LABEL="profile=$PROFILE features='${COMBO:-<default>}'"

    hdr "cargo check   [$LABEL]"
    # shellcheck disable=SC2086
    timeout 600 cargo check $PFLAG $COMBO --all-targets 2>&1 | tail -5
    [[ ${PIPESTATUS[0]} -ne 0 ]] && { echo "CHECK FAILED [$LABEL]"; FAIL=1; continue; }

    hdr "cargo build   [$LABEL]   (mandatory: refreshes the cdylib)"
    # shellcheck disable=SC2086
    timeout 600 cargo build $PFLAG $COMBO 2>&1 | tail -5
    [[ ${PIPESTATUS[0]} -ne 0 ]] && { echo "BUILD FAILED [$LABEL]"; FAIL=1; continue; }

    hdr "symbol parity [$LABEL]"
    R_SO=$( [[ $PROFILE == release ]] \
            && echo "$ROOT/translation/target/release/libtritanopia_lib.so" \
            || echo "$ROOT/translation/target/debug/libtritanopia_lib.so" )
    [[ -f "$R_SO" ]] || { echo "MISSING Rust .so: $R_SO"; FAIL=1; continue; }
    # Use a writable scratch dir: /tmp is read-only in some sandboxes, and
    # redirecting there silently produced an empty diff (a false "OK").
    SCRATCH="$ROOT/translation/target/symcheck"; mkdir -p "$SCRATCH" || { echo "no scratch dir"; exit 1; }
    nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u > "$SCRATCH/c_syms" || { echo "nm on C failed"; FAIL=1; continue; }
    nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u > "$SCRATCH/r_syms" || { echo "nm on Rust failed"; FAIL=1; continue; }
    NC=$(wc -l < "$SCRATCH/c_syms"); NR_=$(wc -l < "$SCRATCH/r_syms")
    # A zero C symbol count means the check itself is broken, not that it passed.
    if [[ "$NC" -eq 0 ]]; then
      echo "BROKEN CHECK: nm found 0 exported symbols in $C_SO"; FAIL=1; continue
    fi
    MISSING=$(comm -23 "$SCRATCH/c_syms" "$SCRATCH/r_syms")
    if [[ -n "$MISSING" ]]; then
      echo "MISSING FROM RUST .so:"; echo "$MISSING"; FAIL=1
    else
      echo "OK: all $NC C symbol(s) present in the Rust .so (Rust exports $NR_)"
      echo "    C symbols: $(tr '\n' ' ' < "$SCRATCH/c_syms")"
    fi

    hdr "cargo test    [$LABEL]"
    # shellcheck disable=SC2086
    timeout 600 cargo test $PFLAG $COMBO 2>&1 | tail -25
    [[ ${PIPESTATUS[0]} -ne 0 ]] && { echo "TESTS FAILED [$LABEL]"; FAIL=1; }
  done
done

hdr "RESULT"
if [[ $FAIL -eq 0 ]]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit $FAIL

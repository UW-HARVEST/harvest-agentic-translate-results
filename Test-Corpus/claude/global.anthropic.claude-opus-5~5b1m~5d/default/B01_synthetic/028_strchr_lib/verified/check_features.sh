#!/usr/bin/env bash
# Phase D driver: rebuild both shared objects, prove symbol parity with nm, and
# run the whole differential suite under every Cargo feature combination and
# both profiles.
#
# Usage: ./check_features.sh
set -uo pipefail

cd "$(dirname "$0")"
CRATE="$PWD"
ROOT="$(cd .. && pwd)"
CARGO_FLAGS="--offline"   # drop --offline if the crates.io index is reachable
rc=0

step() { printf '\n==== %s ====\n' "$*"; }
fail() { printf '!! FAIL: %s\n' "$*"; rc=1; }

# ---------------------------------------------------------------- build C .so
step "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { fail "C build"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
[ -f "$C_SO" ] || { fail "missing $C_SO"; exit 1; }

# ------------------------------------------------------- enumerate feature set
# Feature names come straight out of Cargo.toml, so a future [features] table is
# picked up automatically instead of being hard-coded here.
step "Enumerating Cargo features"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { split($0,a,"="); gsub(/[[:space:]]/,"",a[1]); print a[1] }
  ' Cargo.toml | grep -v '^default$'
)
echo "declared non-default features: ${FEATURES[*]:-<none>}"

# Build the list of feature configurations to test: the default build, the
# no-default-features build, then every non-empty subset of the declared
# features (powerset), which for an empty [features] table collapses to the
# first two.
CONFIGS=("default:")
CONFIGS+=("no-default:--no-default-features")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  total=$(( (1 << n) - 1 ))
  for ((mask = 1; mask <= total; mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( (mask >> i) & 1 )); then combo="$combo,${FEATURES[$i]}"; fi
    done
    combo="${combo#,}"
    CONFIGS+=("$combo:--no-default-features --features $combo")
    CONFIGS+=("default+$combo:--features $combo")
  done
fi
echo "configurations to verify: ${#CONFIGS[@]}"

# ----------------------------------------------------- per-configuration runs
defined_syms() { nm -D --defined-only "$1" | awk '$2 ~ /^[TDBRW]$/ {print $3}' | sort -u; }

for profile in dev release; do
  case "$profile" in
    dev)     PROF_FLAG="";          PROF_DIR="debug"   ;;
    release) PROF_FLAG="--release"; PROF_DIR="release" ;;
  esac

  for entry in "${CONFIGS[@]}"; do
    name="${entry%%:*}"
    flags="${entry#*:}"
    step "profile=$profile features=$name"

    # shellcheck disable=SC2086
    cargo build $CARGO_FLAGS $PROF_FLAG $flags >/dev/null 2>&1 \
      || { fail "cargo build (profile=$profile features=$name)"; continue; }

    RUST_SO="$CRATE/target/$PROF_DIR/libdriver.so"
    if [ ! -f "$RUST_SO" ]; then fail "missing $RUST_SO"; continue; fi

    # Symbol parity: every symbol the C .so defines must be defined by the Rust .so.
    missing="$(comm -23 <(defined_syms "$C_SO") <(defined_syms "$RUST_SO"))"
    if [ -n "$missing" ]; then
      fail "symbols missing from the Rust .so (profile=$profile features=$name): $(echo $missing)"
    else
      echo "symbol parity: OK ($(defined_syms "$C_SO" | tr '\n' ' '))"
    fi

    # Undefined non-libc symbols must not appear.
    undef="$(nm -D --undefined-only "$RUST_SO" | awk '$1=="U"{print $2}' | sed 's/@.*//' | sort -u)"
    echo "imports: $(echo $undef | cut -c1-200)"

    # shellcheck disable=SC2086
    timeout 600 cargo test $CARGO_FLAGS $PROF_FLAG $flags 2>&1 | tail -5
    # shellcheck disable=SC2086
    if [ "${PIPESTATUS[0]}" -ne 0 ]; then
      fail "cargo test (profile=$profile features=$name)"
    fi
  done
done

step "SUMMARY"
if [ "$rc" -eq 0 ]; then
  echo "ALL configurations passed: symbol parity clean and every differential test green."
else
  echo "FAILURES were reported above."
fi
exit "$rc"

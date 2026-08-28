#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination,
# and assert exported-symbol parity with the C .so for each one.
#
#   ./check_features.sh
#
# Feature combinations are extracted mechanically from Cargo.toml's [features]
# table; the powerset is enumerated (plus the implicit "default" and
# "--no-default-features" builds). This crate currently declares no features, so
# the sweep is: {default}, {no-default-features}.

set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(pwd)"
C_BUILD="$ROOT/../c_src/build"
CARGO_FLAGS="--offline"
TMP="${TMPDIR:-/tmp}"
LOG="$TMP/check_features.$$.log"

fail=0
note() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
bad()  { printf '\033[31mFAIL\033[0m %s\n' "$*"; fail=1; }
good() { printf '\033[32mok\033[0m   %s\n' "$*"; }

# ---------------------------------------------------------------------------
# 0. Build the C shared object exactly as documented.
# ---------------------------------------------------------------------------
note "building the C .so (as documented: no CMAKE_BUILD_TYPE => -O0)"
mkdir -p "$C_BUILD"
( cd "$C_BUILD" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO="$(find "$C_BUILD" -maxdepth 1 -name '*.so' | sort | head -1)"
[ -n "$C_SO" ] || { bad "no C .so produced"; exit 1; }
good "C .so = $C_SO"

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[ \t]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
NF_COUNT=${#FEATURES[@]}
note "declared non-default features: ${NF_COUNT} (${FEATURES[*]:-<none>})"

# powerset of FEATURES, as --features arguments
COMBOS=()
if [ "$NF_COUNT" -eq 0 ]; then
  COMBOS+=("")                        # default build
  COMBOS+=("--no-default-features")   # identical here, but exercised anyway
else
  total=$((1 << NF_COUNT))
  for ((m = 0; m < total; m++)); do
    sel=()
    for ((i = 0; i < NF_COUNT; i++)); do
      (( (m >> i) & 1 )) && sel+=("${FEATURES[$i]}")
    done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("--no-default-features --features $joined")
    COMBOS+=("--features $joined")
  done
fi
note "feature combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 2. For each combination: build (debug + release), diff symbols, run tests.
# ---------------------------------------------------------------------------
c_syms=$(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort)

for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  note "combination: $label"

  # shellcheck disable=SC2086
  if ! timeout 600 cargo build $CARGO_FLAGS --release $combo >/dev/null 2>&1; then
    bad "$label : cargo build --release"; continue
  fi
  # shellcheck disable=SC2086
  if ! timeout 600 cargo build $CARGO_FLAGS $combo >/dev/null 2>&1; then
    bad "$label : cargo build (debug)"; continue
  fi
  # shellcheck disable=SC2086
  if ! timeout 600 cargo check $CARGO_FLAGS --all-targets $combo >/dev/null 2>&1; then
    bad "$label : cargo check --all-targets"; continue
  fi

  for prof in release debug; do
    so="target/$prof/libcircle_collide_lib.so"
    [ -f "$so" ] || { bad "$label : missing $so"; continue; }
    r_syms=$(nm -D --defined-only "$so" | awk '{print $3}' | sort)
    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
    if [ -n "$missing" ]; then
      bad "$label / $prof : symbols missing from the Rust .so:"
      echo "$missing" | sed 's/^/       /'
    else
      good "$label / $prof : symbol parity ($(echo "$c_syms" | wc -l) symbols)"
    fi
    # no undefined non-libc symbols
    undef=$(nm -D --undefined-only "$so" | awk '{print $NF}' \
            | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^$' || true)
    if [ -n "$undef" ]; then
      bad "$label / $prof : undefined non-libc symbols: $undef"
    fi
  done

  # NOTE: capture to a file and inspect it separately. Piping into `grep -q`
  # under `set -o pipefail` would make the pipeline inherit cargo's non-zero
  # exit status and thus report a FAILING run as passing.
  # shellcheck disable=SC2086
  timeout 600 cargo test $CARGO_FLAGS --release $combo >"$LOG" 2>&1
  rc=$?
  if [ "$rc" -ne 0 ] || grep -qE '^test result: FAILED|^error(\[|:)' "$LOG"; then
    bad "$label : cargo test --release (exit $rc)"
    grep -E 'DIVERGENCE|panicked|^test result|^error' "$LOG" | head -40
  else
    good "$label : $(grep -c '\.\.\. ok' "$LOG") tests passed"
  fi
  rm -f "$LOG"
done

note "summary"
if [ "$fail" -eq 0 ]; then
  printf '\033[32mALL FEATURE COMBINATIONS VERIFIED\033[0m\n'
else
  printf '\033[31mVERIFICATION FAILED\033[0m\n'
fi
exit "$fail"

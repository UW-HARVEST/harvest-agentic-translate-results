#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination + both profiles.
#
#   ./check_features.sh
#
# Exits non-zero on the first failure.
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
C_SO="$ROOT/c_src/build/libdriver.so"
CARGO="cargo --offline"
fail=0

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
bad() { printf '\033[31mFAIL: %s\033[0m\n' "$*"; fail=1; }
ok()  { printf '\033[32mok: %s\033[0m\n' "$*"; }

# ---------------------------------------------------------------------------
say "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
ok "$C_SO"

# ---------------------------------------------------------------------------
# Enumerate every feature combination declared in Cargo.toml.
# ---------------------------------------------------------------------------
say "Enumerating feature combinations from Cargo.toml"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "" && a[1] != "default") print a[1] }
  ' "$CRATE_DIR/Cargo.toml"
)
NFEAT=${#FEATURES[@]}
echo "declared non-default features: ${NFEAT} ${FEATURES[*]:-(none)}"

COMBOS=()
if [ "$NFEAT" -eq 0 ]; then
  # No [features] table -> the only configurations that exist.
  COMBOS+=("")                       # default
  COMBOS+=("--no-default-features")
  COMBOS+=("--all-features")
else
  # Full powerset, with and without default features.
  total=$((1 << NFEAT))
  for ((mask = 0; mask < total; mask++)); do
    combo=""
    for ((b = 0; b < NFEAT; b++)); do
      if (((mask >> b) & 1)); then combo="${combo:+$combo,}${FEATURES[$b]}"; fi
    done
    COMBOS+=("--no-default-features${combo:+ --features $combo}")
    COMBOS+=("${combo:+--features $combo}")
  done
  COMBOS+=("--all-features")
fi
echo "will exercise ${#COMBOS[@]} configuration(s)"

# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"

  say "cargo check  [$label]"
  # shellcheck disable=SC2086
  if ( cd "$CRATE_DIR" && $CARGO check $combo --all-targets 2>&1 | tail -5 ); then
    ok "check [$label]"
  else
    bad "check [$label]"; continue
  fi

  for profile in release debug; do
    say "build + symbol parity + tests  [$label] [$profile]"
    relflag=""; [ "$profile" = release ] && relflag="--release"

    # shellcheck disable=SC2086
    ( cd "$CRATE_DIR" && $CARGO build $relflag $combo 2>&1 | tail -3 ) \
      || { bad "build [$label][$profile]"; continue; }

    RUST_SO="$CRATE_DIR/target/$profile/libdriver.so"
    if [ ! -f "$RUST_SO" ]; then bad "missing $RUST_SO"; continue; fi

    # --- symbol parity -----------------------------------------------------
    cdefs=$(nm -D --defined-only "$C_SO" | awk '$2=="T"||$2=="W"{print $3}' | sort -u)
    missing=""
    while read -r sym; do
      [ -z "$sym" ] && continue
      if ! nm -D --defined-only "$RUST_SO" | awk '{print $3}' | grep -qx -- "$sym"; then
        missing="$missing $sym"
      fi
    done <<< "$cdefs"
    if [ -n "$missing" ]; then
      bad "symbols missing from Rust [$label][$profile]:$missing"
    else
      ok "symbol parity ($(echo "$cdefs" | wc -l) C symbols all present) [$label][$profile]"
    fi

    # --- undefined non-libc symbols in the Rust .so ------------------------
    undef=$(nm -D --undefined-only "$RUST_SO" | awk '{print $NF}' \
            | grep -v -E '@GLIBC|@GCC|^_ITM_|^__gmon_start__$' || true)
    if [ -n "$undef" ]; then
      bad "unresolved non-libc symbols in Rust .so [$label][$profile]: $undef"
    else
      ok "0 unresolved non-libc symbols [$label][$profile]"
    fi

    # --- differential tests against THIS .so -------------------------------
    # shellcheck disable=SC2086
    if ( cd "$CRATE_DIR" && DIFF_RUST_SO="$RUST_SO" \
           $CARGO test $relflag $combo 2>&1 | grep -E 'test result|^error' ); then
      ok "tests [$label][$profile]"
    else
      bad "tests [$label][$profile]"
    fi
  done
done

# ---------------------------------------------------------------------------
say "SUMMARY"
if [ "$fail" -eq 0 ]; then
  printf '\033[32mALL CONFIGURATIONS PASSED\033[0m\n'
else
  printf '\033[31mFAILURES PRESENT\033[0m\n'
fi
exit "$fail"

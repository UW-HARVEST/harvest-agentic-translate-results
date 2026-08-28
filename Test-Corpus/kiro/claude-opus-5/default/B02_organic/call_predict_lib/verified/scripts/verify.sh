#!/usr/bin/env bash
# Differential verification driver.
#
# Enumerates every build-time configuration and runs the whole differential
# suite for each one:
#
#   1. Cargo feature combinations, read out of Cargo.toml (there is currently
#      no [features] table, so the only combination is the empty one).
#   2. Cargo profiles: dev and release -- the release cdylib is what actually
#      ships, and its symbol table must match the C library's.
#   3. Optimisation levels for the C reference and the Rust harness. Neither
#      c_src/CMakeLists.txt nor rustc pin a level, and `call_predict` compares
#      *function addresses*, so identical-code folding is a real hazard here.
#
# Usage: scripts/verify.sh [quick]
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
LOG_DIR="target/difftest-logs"
mkdir -p "$LOG_DIR"

FAILED=0
PASSED=0

# ---------------------------------------------------------------------------
# 1. Feature combinations (powerset of the [features] table).
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)

COMBOS=()
n=${#FEATURES[@]}
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=""
  for ((b = 0; b < n; b++)); do
    if ((mask & (1 << b))); then
      combo="${combo:+$combo,}${FEATURES[b]}"
    fi
  done
  COMBOS+=("$combo")
done
# Also cover the crate's declared defaults.
COMBOS+=("__default__")

echo "features declared: ${n} (${FEATURES[*]:-none})"
echo "feature combinations to test: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 2 & 3. Profiles x optimisation levels.
# ---------------------------------------------------------------------------
PROFILES=("dev" "release")
if [[ "${1:-}" == "quick" ]]; then
  OPTS=("-O0|")
else
  OPTS=(
    "-O0|"
    "-O1|-O"
    "-O2|-C opt-level=2"
    "-O3|-C opt-level=3"
    "-Os|-C opt-level=s"
    "-O2 -fno-inline|-C opt-level=2 -C inline-threshold=0"
  )
fi

run_one() {
  local combo="$1" profile="$2" cflags="$3" rustflags="$4"
  local feat_args=()
  case "$combo" in
    __default__) feat_args=() ;;
    "")          feat_args=(--no-default-features) ;;
    *)           feat_args=(--no-default-features --features "$combo") ;;
  esac
  local prof_args=()
  [[ "$profile" == "release" ]] && prof_args=(--release)

  local tag
  tag="feat=${combo:-none}_prof=${profile}_c=${cflags// /}_r=${rustflags// /}"
  tag="${tag//\//}"
  local log="$LOG_DIR/${tag}.log"

  DIFFTEST_CFLAGS="$cflags" DIFFTEST_RUSTFLAGS="$rustflags" \
    timeout 600 cargo test "${feat_args[@]}" "${prof_args[@]}" \
    >"$log" 2>&1
  local rc=$?

  if ((rc == 0)); then
    PASSED=$((PASSED + 1))
    printf 'PASS  %s\n' "$tag"
  else
    FAILED=$((FAILED + 1))
    printf 'FAIL  %s   (rc=%d, log: %s)\n' "$tag" "$rc" "$log"
    grep -E 'panicked|assertion|error(\[|:)' "$log" | head -8
  fi
}

for combo in "${COMBOS[@]}"; do
  for profile in "${PROFILES[@]}"; do
    for pair in "${OPTS[@]}"; do
      run_one "$combo" "$profile" "${pair%%|*}" "${pair##*|}"
    done
  done
done

echo
echo "=================================================="
echo "configurations passed: $PASSED   failed: $FAILED"
echo "=================================================="
((FAILED == 0))

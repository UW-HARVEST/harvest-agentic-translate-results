#!/usr/bin/env bash
# Copyright 2025 MIT Lincoln Laboratory
# SPDX-License-Identifier: MIT
#
# Full verification sweep for the C -> Rust translation.
#
#   1. builds the C reference with CMake, exactly as documented in c_src/
#   2. enumerates every valid Cargo feature combination straight out of
#      Cargo.toml (the power set of [features]) and `cargo check`s each one
#   3. runs the whole differential suite for every combination, in both the dev
#      and the release profile (release sets `panic = "abort"`, so it is a
#      genuinely different build - CONFIGS.md row 40)
#
# Usage: scripts/verify.sh [--quick]
#   --quick  skip the release-profile pass

set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT=$PWD
LOGDIR=${TMPDIR:-/tmp}/driver_verify.$$
mkdir -p "$LOGDIR"

QUICK=0
[[ "${1:-}" == "--quick" ]] && QUICK=1

FAILED=0
step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '   \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '   \033[31mFAIL\033[0m %s\n' "$*"; FAILED=1; }

run() { # run <label> <logfile> <cmd...>
  local label=$1 log=$2; shift 2
  if timeout 600 "$@" >"$log" 2>&1; then
    ok "$label"
  else
    bad "$label  (see $log)"
    tail -n 25 "$log" | sed 's/^/        /'
  fi
}

# ---------------------------------------------------------------------------
step "1. Build the C reference with CMake"
# ---------------------------------------------------------------------------
mkdir -p c_src/build
if (cd c_src/build \
      && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      && timeout 600 cmake --build .) >"$LOGDIR/cmake.log" 2>&1; then
  ok "cmake build -> c_src/build/driver"
else
  bad "cmake build (see $LOGDIR/cmake.log)"
  tail -n 25 "$LOGDIR/cmake.log" | sed 's/^/        /'
fi

# ---------------------------------------------------------------------------
step "2. Enumerate feature combinations from Cargo.toml"
# ---------------------------------------------------------------------------
# Pull the feature names out of the [features] table, ignoring comments.
mapfile -t FEATURES < <(
  awk '
    /^[[:space:]]*\[features\]/ { inside = 1; next }
    /^[[:space:]]*\[/           { inside = 0 }
    inside && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, ""); gsub(/[[:space:]]/, ""); print
    }
  ' Cargo.toml
)

N=${#FEATURES[@]}
echo "   features declared: $N ${FEATURES[*]:-(none)}"

# Power set of the feature list. With no features this yields exactly one
# combination: the empty one.
COMBOS=()
for ((mask = 0; mask < (1 << N); mask++)); do
  combo=""
  for ((i = 0; i < N; i++)); do
    if ((mask & (1 << i))); then
      combo="${combo:+$combo,}${FEATURES[$i]}"
    fi
  done
  COMBOS+=("$combo")
done
echo "   combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "     - '${c:-<none>}'"; done

# ---------------------------------------------------------------------------
step "3. cargo check every combination (all targets)"
# ---------------------------------------------------------------------------
i=0
for combo in "${COMBOS[@]}"; do
  i=$((i + 1))
  label="check --no-default-features --features '${combo:-<none>}'"
  run "$label" "$LOGDIR/check.$i.log" \
    cargo check --offline --all-targets --no-default-features --features "$combo"
done
# The default-feature build is a distinct configuration in Cargo's eyes.
run "check (default features)" "$LOGDIR/check.default.log" \
  cargo check --offline --all-targets

# ---------------------------------------------------------------------------
step "4. Differential test suite, every combination x every profile"
# ---------------------------------------------------------------------------
PROFILES=(dev)
((QUICK)) || PROFILES+=(release)

i=0
for combo in "${COMBOS[@]}"; do
  i=$((i + 1))
  for profile in "${PROFILES[@]}"; do
    args=(cargo test --offline --no-default-features --features "$combo")
    [[ $profile == release ]] && args+=(--release)
    label="test [$profile] features='${combo:-<none>}'"
    # NOTE: plain `cargo test` (no --test filter) is required, because example
    # targets - which is where the Rust cdylib lives - are only rebuilt by an
    # unfiltered invocation. tests/common/mod.rs also enforces this with a
    # staleness guard.
    run "$label" "$LOGDIR/test.$i.$profile.log" "${args[@]}"
    grep -hoE 'test result: ok\. [0-9]+ passed' "$LOGDIR/test.$i.$profile.log" \
      | awk '{s += $4} END { if (s) printf "        %d assertions-groups passed\n", s }'
  done
done

# Default-feature run as well.
for profile in "${PROFILES[@]}"; do
  args=(cargo test --offline)
  [[ $profile == release ]] && args+=(--release)
  run "test [$profile] (default features)" "$LOGDIR/test.default.$profile.log" "${args[@]}"
done

# ---------------------------------------------------------------------------
step "Summary"
# ---------------------------------------------------------------------------
if ((FAILED)); then
  printf '\033[31mVERIFICATION FAILED\033[0m  logs in %s\n' "$LOGDIR"
  exit 1
fi
printf '\033[32mVERIFICATION PASSED\033[0m  (%d feature combination(s) x %d profile(s))\n' \
  "${#COMBOS[@]}" "${#PROFILES[@]}"
rm -rf "$LOGDIR"
exit 0

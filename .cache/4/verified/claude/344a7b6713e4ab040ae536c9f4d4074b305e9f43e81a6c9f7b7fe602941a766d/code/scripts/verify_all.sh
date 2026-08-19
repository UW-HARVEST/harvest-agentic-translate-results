#!/usr/bin/env bash
# Full verification matrix for the C-to-Rust translation.
#
#   * enumerates every cargo feature combination (the powerset of [features]),
#   * `cargo check`s each one,
#   * builds the C shared object and the C executable,
#   * runs the differential test suite for each feature combination against a
#     Rust cdylib built with the dev *and* the release flag set
#     (`[profile.release]` sets panic = "abort"), and in the release cargo
#     profile as well.
#
# Usage: scripts/verify_all.sh [--quick]
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT=$(pwd)
CARGO_FLAGS=(--offline)
FAILED=0
SUMMARY=()

log()  { printf '\n\033[1m== %s\033[0m\n' "$*"; }
note() { printf '   %s\n' "$*"; }

record() { # record <ok|FAIL> <description>
  SUMMARY+=("$1 $2")
  [ "$1" = FAIL ] && FAILED=$((FAILED + 1))
  return 0
}

run() { # run <description> <cmd...>
  local desc=$1; shift
  note "\$ $*"
  if timeout 600 "$@" > "${TMPDIR:-/tmp}/verify.log" 2>&1; then
    record ok "$desc"
  else
    record FAIL "$desc"
    tail -n 40 "${TMPDIR:-/tmp}/verify.log"
  fi
}

# ---------------------------------------------------------------------------
# 1. enumerate feature combinations (powerset of [features], minus "default")
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[ \t]*=/ {
      split($0, a, "=");
      gsub(/[ \t]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)
log "feature axes"
if [ "${#FEATURES[@]}" -eq 0 ]; then
  note "Cargo.toml declares no [features]; the only combination is the empty one"
else
  note "features: ${FEATURES[*]}"
fi

COMBOS=("")                        # the empty combination
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( (mask >> i) & 1 )); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
note "${#COMBOS[@]} combination(s) to verify"

# ---------------------------------------------------------------------------
# 2. cargo check every combination (plus the default-features build)
# ---------------------------------------------------------------------------
log "cargo check"
run "check (default features)" cargo check "${CARGO_FLAGS[@]}" --all-targets
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    run "check --no-default-features" \
      cargo check "${CARGO_FLAGS[@]}" --no-default-features --all-targets
  else
    run "check --no-default-features --features $combo" \
      cargo check "${CARGO_FLAGS[@]}" --no-default-features --features "$combo" --all-targets
  fi
done

# ---------------------------------------------------------------------------
# 3. build the C artefacts (CMake executable + shared object)
# ---------------------------------------------------------------------------
log "C build"
mkdir -p c_src/build
( cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . ) \
  > "${TMPDIR:-/tmp}/cmake.log" 2>&1 \
  && record ok "cmake build (C executable)" \
  || { record FAIL "cmake build (C executable)"; tail -n 20 "${TMPDIR:-/tmp}/cmake.log"; }
mkdir -p build_c
run "gcc -shared (C shared object)" \
  gcc -shared -fPIC -o build_c/libdriver_c.so c_src/src/main.c

# ---------------------------------------------------------------------------
# 4. build the cargo artefacts so the symbol test sees them, then run the suite
#    for every combination x every Rust cdylib flag set.
# ---------------------------------------------------------------------------
log "cargo build (both profiles, so target/{debug,release}/libdriver.so exist)"
run "build (dev)"     cargo build "${CARGO_FLAGS[@]}"
run "build (release)" cargo build "${CARGO_FLAGS[@]}" --release

log "differential test suite"
for combo in "${COMBOS[@]}"; do
  for so_profile in dev release; do
    for cargo_profile in dev release; do
      args=(test "${CARGO_FLAGS[@]}")
      desc="tests"
      if [ -z "$combo" ]; then
        args+=(--no-default-features); desc="$desc [no-default-features]"
      else
        args+=(--no-default-features --features "$combo"); desc="$desc [features=$combo]"
      fi
      [ "$cargo_profile" = release ] && { args+=(--release); }
      desc="$desc cargo=$cargo_profile so=$so_profile"
      note "\$ RUST_SO_PROFILE=$so_profile cargo ${args[*]} -- --test-threads=1"
      if RUST_SO_PROFILE=$so_profile timeout 600 cargo "${args[@]}" -- --test-threads=1 \
           > "${TMPDIR:-/tmp}/verify.log" 2>&1; then
        record ok "$desc"
        grep -E "^test result:" "${TMPDIR:-/tmp}/verify.log" | sed 's/^/     /'
      else
        record FAIL "$desc"
        tail -n 40 "${TMPDIR:-/tmp}/verify.log"
      fi
    done
  done
done

# also once with the default feature set, in parallel-test mode
log "default feature set, parallel tests"
run "tests [default features] parallel" cargo test "${CARGO_FLAGS[@]}"

# ---------------------------------------------------------------------------
# 5. summary
# ---------------------------------------------------------------------------
log "summary"
for line in "${SUMMARY[@]}"; do
  case "$line" in
    ok*)   printf '   \033[32m%s\033[0m\n' "$line" ;;
    FAIL*) printf '   \033[31m%s\033[0m\n' "$line" ;;
  esac
done
if [ "$FAILED" -eq 0 ]; then
  printf '\n\033[32mALL %d STEPS PASSED\033[0m\n' "${#SUMMARY[@]}"
else
  printf '\n\033[31m%d of %d STEPS FAILED\033[0m\n' "$FAILED" "${#SUMMARY[@]}"
fi
exit "$FAILED"

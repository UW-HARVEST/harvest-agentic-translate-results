#!/usr/bin/env bash
# Full verification sweep: builds the C and Rust shared objects, then runs every
# test target under every crate feature combination and under both profiles.
#
#   scripts/verify_all.sh            # run everything, log to verify_all.log
#
# Exits non-zero if any configuration fails.
set -uo pipefail

cd "$(dirname "$0")/.."
ROOT="$(cd .. && pwd)"
LOG="$PWD/verify_all.log"
: > "$LOG"
FAIL=0

say() { echo "$@" | tee -a "$LOG"; }

say "=== building the C shared library ==="
if ! ( mkdir -p "$ROOT/c_src/build" \
       && cd "$ROOT/c_src/build" \
       && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
       && cmake --build . ) >>"$LOG" 2>&1; then
  say "C build FAILED (see $LOG)"; exit 1
fi
say "C .so: $(ls -l "$ROOT/c_src/build/libdriver.so")"

# ---------------------------------------------------------------------------
# Enumerate the crate's feature powerset straight out of Cargo.toml.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {sub(/[[:space:]]*=.*/,""); print}' Cargo.toml
)

declare -a COMBO_LABEL COMBO_FLAGS
add_combo() { COMBO_LABEL+=("$1"); COMBO_FLAGS+=("$2"); }

if [ "${#FEATURES[@]}" -eq 0 ]; then
  say "=== Cargo.toml declares no [features]; the powerset collapses to one configuration ==="
  add_combo "default (no features declared)" ""
  add_combo "--no-default-features"          "--no-default-features"
else
  say "=== features found: ${FEATURES[*]} ==="
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    if [ -z "$combo" ]; then
      add_combo "--no-default-features" "--no-default-features"
    else
      add_combo "--no-default-features --features $combo" "--no-default-features --features $combo"
    fi
  done
  add_combo "default" ""
fi

for profile in debug release; do
  pflag=""
  [ "$profile" = release ] && pflag="--release"

  for idx in "${!COMBO_LABEL[@]}"; do
    label="${COMBO_LABEL[$idx]}"
    # shellcheck disable=SC2206
    read -r -a fflags <<< "${COMBO_FLAGS[$idx]}"
    say ""
    say "########################################################"
    say "### profile=${profile}  features=[${label}]"
    say "########################################################"

    if ! cargo build --offline ${pflag:+$pflag} "${fflags[@]}" >>"$LOG" 2>&1; then
      say "!!! BUILD FAILED (${profile}, ${label})"; FAIL=1; continue
    fi
    if timeout 600 cargo test --offline ${pflag:+$pflag} "${fflags[@]}" >>"$LOG" 2>&1; then
      say ">>> PASS (${profile}, ${label})"
      grep -E 'test result' "$LOG" | tail -4 | sed 's/^/      /' | tee -a "$LOG" >/dev/null
    else
      say "!!! TESTS FAILED (${profile}, ${label}) — see $LOG"; FAIL=1
    fi
  done
done

say ""
if [ "$FAIL" -eq 0 ]; then
  say "================ ALL CONFIGURATIONS PASSED ================"
else
  say "================ SOME CONFIGURATIONS FAILED ==============="
fi
exit "$FAIL"

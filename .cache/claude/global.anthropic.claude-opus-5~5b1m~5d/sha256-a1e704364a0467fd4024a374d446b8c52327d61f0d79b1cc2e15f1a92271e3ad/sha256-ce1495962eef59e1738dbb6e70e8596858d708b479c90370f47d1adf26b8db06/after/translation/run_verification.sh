#!/usr/bin/env bash
# Full differential verification: every feature combination x every profile.
#
#   ./run_verification.sh          # writes target/verify-logs/*.log + summary.txt
#
# NOTE: `cargo test` does NOT relink a cdylib, so the explicit `cargo build --lib
# --examples` before each test run is mandatory, not cosmetic.  The staleness
# guard in tests/common/mod.rs will refuse to run against an out-of-date .so.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
CARGO_FLAGS="${CARGO_FLAGS:---offline}"
LOGS="$HERE/target/verify-logs"
SUMMARY="$LOGS/summary.txt"
rm -rf "$LOGS"; mkdir -p "$LOGS"
: > "$SUMMARY"
FAIL=0

log() { echo "$*" | tee -a "$SUMMARY"; }

log "=============================================================="
log " 1. Build the C ground-truth shared library"
log "=============================================================="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) > "$LOGS/c-build.log" 2>&1 \
  || { log "C build FAILED (see $LOGS/c-build.log)"; exit 1; }
CSO=$(ls -1 "$ROOT/c_src/build"/lib*.so)
log " C .so: $CSO"

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml.  This crate declares no
# [features] table, so the powerset is {default} == {no-default-features} ==
# {all-features}; all three are still exercised explicitly so that adding a
# feature later cannot silently skip a configuration.
# ---------------------------------------------------------------------------
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}' "$HERE/Cargo.toml")
log ""
log "declared [features]: ${FEATURES:-<none>}"

COMBOS=()
COMBOS+=("default|")
COMBOS+=("no-default-features|--no-default-features")
COMBOS+=("all-features|--all-features")
if [ -n "$FEATURES" ]; then
  feats=($FEATURES); n=${#feats[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    sel=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then sel="${sel:+$sel,}${feats[$i]}"; fi
    done
    COMBOS+=("nodefault+$sel|--no-default-features --features $sel")
  done
fi
log "feature combinations to verify: ${#COMBOS[@]}"

log ""
log "=============================================================="
log " 2. cargo check (--all-targets) for every feature combination"
log "=============================================================="
for combo in "${COMBOS[@]}"; do
  label="${combo%%|*}"; flags="${combo#*|}"
  if ( cd "$HERE" && cargo check $CARGO_FLAGS $flags --all-targets ) \
        > "$LOGS/check-$label.log" 2>&1; then
    log "  $(printf '%-24s' "$label") OK"
  else
    log "  $(printf '%-24s' "$label") FAILED (see $LOGS/check-$label.log)"; FAIL=1
  fi
done

log ""
log "=============================================================="
log " 3. Differential test suite: profile x feature combination"
log "=============================================================="
for profile in dev release; do
  relflag=""; [ "$profile" = release ] && relflag="--release"
  for combo in "${COMBOS[@]}"; do
    label="${combo%%|*}"; flags="${combo#*|}"
    tag="$profile-$label"
    lf="$LOGS/test-$tag.log"

    # MANDATORY: relink the cdylib and the LD_PRELOAD example before testing.
    if ! ( cd "$HERE" && cargo build $CARGO_FLAGS $relflag $flags --lib --examples ) \
            > "$LOGS/build-$tag.log" 2>&1; then
      log "  $(printf '%-32s' "$tag") BUILD FAILED (see $LOGS/build-$tag.log)"; FAIL=1; continue
    fi

    ( cd "$HERE" && timeout 600 cargo test $CARGO_FLAGS $relflag $flags -- --test-threads=1 ) \
        > "$lf" 2>&1
    rc=$?
    # `bc` is not guaranteed to exist -- sum with awk.
    passed=$(grep -oE '[0-9]+ passed' "$lf" | awk '{s+=$1} END{print s+0}')
    failed=$(grep -oE '[0-9]+ failed' "$lf" | awk '{s+=$1} END{print s+0}')
    suites=$(grep -cE '^test result:' "$lf")
    if [ "$rc" -eq 0 ] && [ "${failed:-1}" -eq 0 ] && [ "${passed:-0}" -gt 0 ]; then
      log "  $(printf '%-32s' "$tag") OK   suites=$suites tests_passed=${passed:-0}"
    else
      log "  $(printf '%-32s' "$tag") FAIL rc=$rc suites=$suites failed=${failed:-?} (see $lf)"; FAIL=1
    fi
  done
done

log ""
log "=============================================================="
if [ "$FAIL" = 0 ]; then
  log " RESULT: ALL CONFIGURATIONS PASSED"
else
  log " RESULT: FAILURES PRESENT -- see $LOGS"
fi
log "=============================================================="
log "logs: $LOGS"
exit $FAIL
